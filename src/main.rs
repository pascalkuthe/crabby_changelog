use std::collections::HashMap;
use std::fs::read_to_string;

use anyhow::{Context, Result};
use clap::Parser;
use tera::Tera;
use xshell::{cmd, Shell};

use crate::config::Config;
use crate::github_api::PullRequest;
use crate::state::{Change, ReleaseState};

mod cli;
mod config;
mod github_api;
mod state;

impl github_api::PullRequest {
    pub fn changelog_entries<'a>(&'a self, config: &'a Config, dst: &mut ReleaseState) {
        let mut main_change = None;

        let mut generate_main_change = true;

        if let Some(body) = &self.body {
            for line in body.lines() {
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
        }

        if main_change.is_none() {
            if let Some(title) = self.title.as_deref() {
                main_change = Some(title.trim().to_owned());
            }
        }

        if let Some(main_change) = main_change {
            if let Some(labels) = &self.labels {
                for label in labels {
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
                .iter()
                .position(|it| it == &change1.group)
                .unwrap_or(config.groups.len());
            let pos2 = config
                .groups
                .iter()
                .position(|it| it == &change2.group)
                .unwrap_or(config.groups.len());
            pos1.cmp(&pos2)
        });

        let mut ctx = tera::Context::from_serialize(ctx)?;
        ctx.insert("version", &version);
        ctx.insert("repo", &config.repo);
        tera.register_filter("upper_first", upper_first_filter);
        tera.register_function("pr_url", make_pr_url(config.repo.clone()));
        let res = tera.render("template", &ctx)?;
        Ok(res)
    }
}

/// Filter for making the first character of a string uppercase.
fn upper_first_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let mut s = tera::try_get_value!("upper_first_filter", "value", String, value);
    let mut c = s.chars();
    s = match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    };
    Ok(tera::to_value(&s)?)
}

fn make_pr_url(repo: String) -> impl tera::Function {
    Box::new(
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let pr =
                match args.get("pr") {
                    Some(val) => match tera::from_value::<u64>(val.clone()).ok().or_else(|| {
                        tera::from_value::<String>(val.clone())
                            .ok()?
                            .strip_prefix('#')?
                            .parse()
                            .ok()
                    }) {
                        Some(val) => val,
                        None => return Err(
                            "pr_url argument 'pr' must be a a number (optionally prefixed with #)"
                                .into(),
                        ),
                    },
                    None => return Err("pr_url is missing required argument 'pr'".into()),
                };

            let repo = match args.get("repo") {
                Some(val) => match tera::from_value::<String>(val.clone()) {
                    Ok(val) => val,
                    Err(_) => return Err("pr_url argument 'repo' must be a string".into()),
                },
                None => repo.clone(),
            };

            let url = format!("https://github.com/{repo}/pull/{pr}");
            Ok(tera::to_value(&url)?)
        },
    )
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
                Ok(cmd!(sh, "git show -s --format=%ct {git_ref}")
                    .read()?
                    .parse()?)
            })
            .or_else(|| self.since_timestamp.map(Ok));
        if let Some(timestamp) = timestamp {
            let timestamp = timestamp?;
            PullRequest::repo_query(
                &config.repo,
                &config.main_branch,
                Some(github_api::IssueState::Closed),
                Some(github_api::Sort::Updated),
            )?
            .filter_map(|pr| {
                pr.map(|pr| {
                    let merged_at = pr.merged_at?;
                    Some((pr, merged_at))
                })
                .transpose()
            })
            .take_while(|pr| {
                pr.as_ref().map_or(true, |(_, merged_at)| {
                    merged_at.timestamp() as u32 > timestamp
                })
            })
            .map(|pr| pr.map(|(pr, _)| pr))
            .collect()
        } else {
            self.ids
                .iter()
                .map(|&pr| PullRequest::lookup(&config.repo, pr))
                .collect()
        }
    }
    pub fn run(&self, config: &Config, state: &mut ReleaseState) -> Result<bool> {
        for &pr in &self.ids {
            let pr = PullRequest::lookup(&config.repo, pr)?;
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
