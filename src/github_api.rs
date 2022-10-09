use anyhow::Result;
use serde::Deserialize;
use std::vec;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum IssueState {
    Open,
    Closed,
}

impl IssueState {
    const fn as_str(self) -> &'static str {
        match self {
            IssueState::Open => "open",
            IssueState::Closed => "close",
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Sort {
    Created,
    Updated,
}

impl Sort {
    const fn as_str(self) -> &'static str {
        match self {
            Sort::Created => "created",
            Sort::Updated => "updated",
        }
    }
}

pub struct PullRequestIter<'a> {
    items: vec::IntoIter<PullRequest>,
    next_page: Option<String>,
    token: &'a str,
}

impl<'a> PullRequestIter<'a> {
    fn new(
        repo: &str,
        dst_branch: &str,
        token: &'a str,
        state: Option<IssueState>,
        sort: Option<Sort>,
    ) -> Result<Self> {
        let mut args = vec![("base", dst_branch)];

        if let Some(state) = state {
            args.push(("state", state.as_str()))
        }

        if let Some(sort) = sort {
            args.push(("sort", sort.as_str()));
            args.push(("direction", "desc"));
        }

        let response = call_github_api_endpoint(repo, token, "pulls", &[], &args)?;
        let (items, next_page) = Self::handle_response(response)?;
        Ok(PullRequestIter {
            items: items.into_iter(),
            next_page,
            token,
        })
    }

    fn handle_response(response: ureq::Response) -> Result<(Vec<PullRequest>, Option<String>)> {
        let next_page = response.header("links").and_then(|links| {
            let links = parse_link_header::parse_with_rel(links).ok()?;
            Some(links.get("next")?.raw_uri.clone())
        });
        let res = serde_json::from_str(&response.into_string()?)?;
        Ok((res, next_page))
    }
}

impl Iterator for PullRequestIter<'_> {
    type Item = Result<PullRequest>;

    fn next(&mut self) -> Option<Result<PullRequest>> {
        loop {
            if let Some(item) = self.items.next() {
                return Some(Ok(item));
            }

            let next_page = self.next_page.take()?;
            match call_github_api(&next_page, self.token, &[], &[]).and_then(Self::handle_response)
            {
                Ok((items, next_page)) => {
                    self.items = items.into_iter();
                    self.next_page = next_page;
                }
                Err(err) => return Some(Err(err)),
            }
        }
    }
}

struct PullsIter {
    pulls: vec::IntoIter<PullRequest>,
    next_page: Option<String>,
}

fn call_github_api_endpoint(
    repo: &str,
    token: &str,
    endpoint: &str,
    headers: &[(&str, &str)],
    queries: &[(&str, &str)],
) -> Result<ureq::Response> {
    let url = format!("https://api.github.com/repos/{repo}/{endpoint}");
    call_github_api(&url, token, headers, queries)
}

fn call_github_api(
    url: &str,
    token: &str,
    headers: &[(&str, &str)],
    queries: &[(&str, &str)],
) -> Result<ureq::Response> {
    let mut request = ureq::get(url)
        .set("Accept", "application/vnd.github+json")
        .set("Authorization", token);
    for (param, value) in headers.iter() {
        request = request.set(param, value)
    }
    for (param, value) in queries.iter() {
        request = request.query(param, value);
    }
    let res = request.call()?;
    Ok(res)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct User {
    pub login: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct PullRequest {
    pub url: String,
    pub id: u64,
    pub number: u64,
    pub title: Option<String>,
    pub labels: Option<Vec<Label>>,
    pub body: Option<String>,
    pub merged_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename(deserialize = "user"))]
    pub author: User, // pub user: Option<Box<User>>,
}

impl PullRequest {
    pub fn lookup(repo: &str, token: &str, number: u32) -> Result<PullRequest> {
        let response = call_github_api_endpoint(repo, token, &format!("pulls/{number}"), &[], &[])?
            .into_string()?;
        let res = serde_json::from_str(&response)?;
        Ok(res)
    }

    pub fn repo_query<'a>(
        repo: &str,
        dst_branch: &str,
        token: &'a str,
        state: Option<IssueState>,
        sort: Option<Sort>,
    ) -> Result<PullRequestIter<'a>> {
        PullRequestIter::new(repo, dst_branch, token, state, sort)
    }

    pub fn title_contains(&self, substr: &str) -> bool {
        if let Some(title) = &self.title {
            title.contains(substr)
        } else {
            false
        }
    }
    pub fn has_label(&self, label: &str) -> bool {
        if let Some(labels) = &self.labels {
            labels.iter().any(|it| it.name == label)
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct Label {
    pub id: u64,
    pub name: String,
}
