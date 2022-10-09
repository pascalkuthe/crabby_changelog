use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use indexmap::IndexSet;
use indoc::formatdoc;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Query<T> {
    pub data: T,
}

#[derive(Deserialize)]
pub struct RepoQueryData {
    pub repository: Repository,
}

#[derive(Deserialize)]
pub struct Repository {
    #[serde(rename = "pullRequest")]
    pub pull_request: PullRequest,
}

#[derive(Deserialize)]
pub struct SearchData<T> {
    pub search: T,
}

#[derive(Deserialize)]
pub struct Nodes<T> {
    pub nodes: Vec<T>,
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
}

#[derive(Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub labels: Nodes<Label>,
    pub author: Author,
}

#[derive(Deserialize)]
pub struct Author {
    pub login: String,
}

#[derive(Deserialize)]
pub struct PageInfo {
    #[serde(rename = "endCursor")]
    pub end_cursor: Option<String>,
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct Label {
    pub name: String,
}

pub fn lookup_pr(repo: &str, pr: u64) -> Result<PullRequest> {
    let (owner, repo) = repo.split_once('/').context("invalid repository name")?;
    let request = formatdoc!(
        "
        {{
         repository(owner: \"{owner}\", name: \"{repo}\") {{
            pullRequest(number: {pr}) {PR_QUERY}    
         }}
        }}"
    );
    let query: Query<RepoQueryData> = call_api(&request)?;
    Ok(query.data.repository.pull_request)
}

pub enum PrFilter {
    Open,
    MergedSince(DateTime<Utc>),
}

pub struct ListPrs<'a> {
    pub max_fetch: u32,
    pub repo: &'a str,
    pub filter: Option<PrFilter>,
    pub ignored_authors: &'a IndexSet<String>,
    pub ignored_labels: &'a IndexSet<String>,
    pub descending: bool,
    pub head: Option<&'a str>,
    pub base: &'a str,
}

impl ListPrs<'_> {
    pub fn run(&self, cursor: Option<&str>) -> Result<Nodes<PullRequest>> {
        let Self {
            max_fetch,
            repo,
            ref filter,
            ignored_authors,
            ignored_labels,
            descending,
            head,
            base,
        } = *self;

        assert!(
            max_fetch <= 1000,
            "Can fetch at most 1000 search results at once"
        );

        let sort = if descending { "desc" } else { "asc" };
        let mut search = format!("repo:{repo} is:pr  base:{base} sort:updated-{sort}");

        if let Some(head) = head {
            format_to!(&mut search, " head:{head}");
        }

        for author in ignored_authors {
            format_to!(&mut search, " -author:{author}");
        }

        for label in ignored_labels {
            format_to!(&mut search, " -label:{label}");
        }

        match filter {
            Some(PrFilter::MergedSince(merged_since)) => {
                format_to!(
                    &mut search,
                    " merged:{}..{}",
                    merged_since.to_rfc3339_opts(SecondsFormat::Millis, true),
                    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
                );
            }
            Some(PrFilter::Open) => search.push_str(" is:open"),
            None => (),
        }

        let cursor = if let Some(cursor) = cursor {
            format!("\nafter: \"{cursor}\"")
        } else {
            String::new()
        };

        let mut pr_query = format!("... on PullRequest {PR_QUERY}");
        pr_query = NODES_QUERY.replace("DATA", &pr_query);

        let query = formatdoc!(
            "
            {{
              search(
                query: \"{search}\",
                type: ISSUE,
                first: {max_fetch}{cursor}
            ){pr_query}    
            }}",
        );
        let res: Query<SearchData<_>> = call_api(&query)?;
        Ok(res.data.search)
    }
}

const PR_QUERY: &str = r#"{
  number
  title
  body
  labels(first: 100) {
    nodes {
      name
    }
    pageInfo {
      endCursor
      hasNextPage
    }
  }
  author {
    login
  }
}
"#;

const NODES_QUERY: &str = r#"{
  nodes {DATA}
  pageInfo {
    endCursor
    hasNextPage
  }
}"#;

#[derive(Serialize)]
struct GraphQlQuery {
    query: String,
}

fn call_api<T: for<'de> Deserialize<'de>>(query: &str) -> Result<T> {
    let token = std::env::var("GITHUB_TOKEN").context("no token set")?;
    let request = ureq::post("https://api.github.com/graphql")
        .set("Accept", "application/vnd.github+json")
        .set("Authorization", &format!("bearer {token}"));
    let query = GraphQlQuery {
        query: query.replace('\n', " "),
    };
    let query = serde_json::to_string(&query)?;
    let res = request.send_string(&query).map_err(|err| match err {
        ureq::Error::Status(status, response) => {
            anyhow!(
                "github api call failed (status {status}):\n{}",
                response.into_string().unwrap_or_default()
            )
        }
        ureq::Error::Transport(transport) => {
            anyhow::Error::from(transport).context("sending request to github api failed")
        }
    })?;
    let res = res.into_reader();
    Ok(serde_json::from_reader(res)?)
}
