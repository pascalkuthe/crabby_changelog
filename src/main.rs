use std::collections::HashMap;
use std::fs::read_to_string;

use anyhow::{Context, Result};
use clap::Parser;
use tera::Tera;

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

        let pr = vec![state::PullRequest {
            number: self.number,
            url: self.url.clone(),
        }];

        let mut generate_main_change = true;

        if let Some(body) = &self.body {
            for line in body.lines() {
                if let Some((_, mut rem)) = line.trim().split_once("changelog") {
                    let mut category = None;
                    if rem.starts_with('[') {
                        if let Some((category_, rem_)) = rem[1..].split_once(']') {
                            category = match config.label_categories.get(category_) {
                                Some(category_) => Some(category_.to_owned()),
                                None => Some(category_.to_owned()),
                            };
                            rem = rem_;
                        }
                    }
                    if !rem.starts_with(':') {
                        continue;
                    }

                    if let Some(category) = category {
                        let change = Change {
                            category,
                            message: rem.trim_start().to_owned(),
                            prs: pr.clone(),
                        };
                        dst.changes.push(change);
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
                    if let Some(category) = config.label_categories.get(&label.name) {
                        dst.changes.push(Change {
                            message: main_change.clone(),
                            category: category.to_owned(),
                            prs: pr.clone(),
                        });
                    }
                }
            } else if generate_main_change {
                if let Some(category) = &config.default_category {
                    dst.changes.push(Change {
                        message: main_change,
                        category: category.to_owned(),
                        prs: pr,
                    });
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
        tera.register_filter("upper_first", upper_first_filter);
        let mut ctx = tera::Context::from_serialize(self)?;
        ctx.insert("version", &version);
        let res = tera.render("template", &ctx)?;
        Ok(res)
    }
}

/// Filter for making the first character of a string uppercase.
fn upper_first_filter(
    value: &tera::Value,
    _: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let mut s = tera::try_get_value!("upper_first_filter", "value", String, value);
    let mut c = s.chars();
    s = match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    };
    Ok(tera::to_value(&s)?)
}

impl cli::Render {
    pub fn run(&self, config: &Config, state: &ReleaseState) -> Result<bool> {
        println!("{}", state.render(config, self.version.as_deref())?);
        Ok(false)
    }
}

impl cli::AddPr {
    pub fn run(&self, config: &Config, state: &mut ReleaseState) -> Result<bool> {
        let token = std::env::var("GITHUB_TOKEN").context("no token set")?;
        for &pr in &self.ids {
            let pr = PullRequest::lookup(&self.repo, &token, pr)?;
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
            serde_json::from_str(&state)?
        };
        let config = read_to_string(&self.config).context("config not found")?;
        let config = toml::from_str(&config)?;

        let state_modified = match self.command {
            cli::Commands::Render(cmd) => cmd.run(&config, &state)?,
            cli::Commands::AddPr(cmd) => cmd.run(&config, &mut state)?,
        };

        if state_modified {
            let state = serde_json::to_string_pretty(&state)?;
            std::fs::write(&self.state, state)?;
        }

        Ok(())
    }
}

fn main() {
    if let Err(err) = cli::CliArgs::parse().run() {
        eprintln!("{err}");
    }
}
