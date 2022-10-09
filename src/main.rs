use std::fs::read_to_string;

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use clap::Parser;
use tera::Tera;
use xshell::{cmd, Shell};

use crate::config::Config;
use crate::github_api::PullRequest;
use crate::state::{Change, ReleaseState};
use crate::tera_functions::{make_pr_list_md, make_pr_md_link, make_pr_url, upper_first_filter};

#[macro_use]
mod util;
mod cli;
mod config;
mod github_api;
mod state;
mod tera_functions;

impl github_api::PullRequest {
    pub fn is_ignored(&self, config: &Config) -> bool {
        for label in &self.labels.nodes {
            if config.ignored_labels.contains(&label.name) {
                return true;
            }
        }

        if config.ignored_authors.contains(&self.author.login) {
            return true;
        }

        for prefix in config.ignored_title_prefix.iter() {
            if self.title.trim().starts_with(prefix) {
                return true;
            }
        }

        false
    }

    pub fn changelog_entries<'a>(&'a self, config: &'a Config, dst: &mut ReleaseState) {
        if self.is_ignored(config) {
            return;
        }

        let mut main_change = None;

        let mut generate_main_change = true;

        for line in self.body.lines() {
            if let Some((_, mut rem)) = line.trim().split_once("changelog") {
                let mut group = None;
                if rem.starts_with('[') {
                    if let Some((group_, rem_)) = rem[1..].split_once(']') {
                        group = match config.label_groups.get(group_) {
                            Some(group_) => Some(group_.to_owned()),
                            None => Some(group_.to_owned()),
                        };
                        rem = rem_;
                    }
                }
                if !rem.starts_with(':') {
                    continue;
                }

                if let Some(group) = group {
                    dst.insert_pr_change(
                        Change {
                            message: rem.trim_start().to_owned(),
                            group,
                        },
                        self.number,
                    );
                } else {
                    main_change = Some(rem.trim_start().to_owned())
                }

                generate_main_change = false;
            }
        }

        if main_change.is_none() {
            main_change = Some(self.title.trim().to_owned());
        }

        if let Some(main_change) = main_change {
            for label in &self.labels.nodes {
                if let Some(group) = config.label_groups.get(&label.name) {
                    generate_main_change = false;
                    dst.insert_pr_change(
                        Change {
                            message: main_change.clone(),
                            group: group.to_owned(),
                        },
                        self.number,
                    );
                }
            }
            if generate_main_change {
                if let Some(group) = &config.default_group {
                    dst.insert_pr_change(
                        Change {
                            message: main_change,
                            group: group.to_owned(),
                        },
                        self.number,
                    );
                }
            }
        }
    }
}

impl ReleaseState {
    pub fn add_pr_changes(&mut self, pr: &PullRequest, config: &Config) {
        pr.changelog_entries(config, self);
        self.authors.insert(pr.author.login.clone());
    }

    pub fn render(&self, config: &Config, version: Option<&str>) -> Result<String> {
        let mut tera = Tera::default();
        tera.add_raw_template("template", &config.template)?;
        let mut ctx = self.to_tera_ctx();
        ctx.changes.sort_by(|change1, _, change2, _| {
            let pos1 = config
                .groups
                .get_index_of(&change1.group)
                .unwrap_or(config.groups.len());
            let pos2 = config
                .groups
                .get_index_of(&change2.group)
                .unwrap_or(config.groups.len());
            pos1.cmp(&pos2)
        });

        let mut ctx = tera::Context::from_serialize(ctx)?;
        ctx.insert("version", &version);
        ctx.insert("repo", &config.repo);
        tera.register_filter("upper_first", upper_first_filter);
        tera.register_function("pr_url", make_pr_url(config.repo.clone()));
        tera.register_function("pr_md_link", make_pr_md_link(config.repo.clone()));
        tera.register_function("pr_list_md", make_pr_list_md(config.repo.clone()));
        let res = tera.render("template", &ctx)?;
        Ok(res)
    }
}

impl cli::Render {
    pub fn run(&self, config: &Config, state: &ReleaseState) -> Result<bool> {
        println!("{}", state.render(config, self.version.as_deref())?);
        Ok(false)
    }
}

impl cli::AddPr {
    pub fn get_prs(&self, config: &Config) -> Result<Vec<PullRequest>> {
        let timestamp = self
            .since_ref
            .as_ref()
            .map(|git_ref| -> Result<_> {
                let sh = Shell::new()?;
                Ok(cmd!(sh, "git log -1 --format=%ct {git_ref}")
                    .read()?
                    .parse()?)
            })
            .or_else(|| self.since_timestamp.map(Ok));
        if let Some(timestamp) = timestamp {
            let timestamp = timestamp.context("failed to obtain timestamp for git rev")? + 10;
            let timestamp = NaiveDateTime::from_timestamp(timestamp as i64, 0);
            let timestamp = DateTime::from_utc(timestamp, Utc);
            let query = github_api::ListPrs {
                max_fetch: 100,
                repo: &config.repo,
                filter: Some(github_api::PrFilter::MergedSince(timestamp)),
                ignored_authors: &config.ignored_authors,
                ignored_labels: &config.ignored_labels,
                descending: false,
                head: None,
                base: &config.main_branch,
            };

            let github_api::Nodes {
                nodes: mut res,
                mut page_info,
            } = query.run(None)?;

            loop {
                if !page_info.has_next_page {
                    break;
                }
                if let Some(cursor) = page_info.end_cursor {
                    let query = query.run(Some(&cursor))?;
                    res.extend(query.nodes);
                    page_info = query.page_info;
                } else {
                    break;
                }
            }
            Ok(res)
        } else {
            self.prs
                .iter()
                .map(|&pr| github_api::lookup_pr(&config.repo, pr))
                .collect()
        }
    }
    pub fn run(&self, config: &Config, state: &mut ReleaseState) -> Result<bool> {
        for pr in self.get_prs(config).context("failed to retrieve PRs")? {
            state.add_pr_changes(&pr, config);
        }
        Ok(true)
    }
}

impl cli::CliArgs {
    fn run(self) -> Result<()> {
        let state = read_to_string(&self.state).unwrap_or_default();
        let mut state = if state.is_empty() {
            println!("statefile not found, generating a new release");
            ReleaseState::default()
        } else {
            toml::from_str(&state)?
        };
        let config = read_to_string(&self.config).context("config not found")?;
        let config = toml::from_str(&config)?;

        let state_modified = match self.command {
            cli::Commands::Render(cmd) => cmd.run(&config, &state)?,
            cli::Commands::AddPr(cmd) => cmd.run(&config, &mut state)?,
        };

        if state_modified {
            let state = toml::to_string_pretty(&state)?;
            std::fs::write(&self.state, state)?;
        }

        Ok(())
    }
}

fn main() {
    if let Err(err) = cli::CliArgs::parse().run() {
        eprintln!("error: {err:?}")
    }
}
