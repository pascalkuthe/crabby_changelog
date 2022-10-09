use std::collections::HashMap;

/// Filter for making the first character of a string uppercase.
pub fn upper_first_filter(
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

pub fn make_pr_url(repo: String) -> impl tera::Function {
    Box::new(
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let repo = extract_repo(&args, &repo)?;
            let pr = extract_pr_from_args(args)?;
            let url = pr_url(pr, &repo);
            Ok(tera::to_value(url)?)
        },
    )
}

pub fn make_pr_list_md(repo: String) -> impl tera::Function {
    Box::new(
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let repo = extract_repo(args, &repo)?;
            let prs = match args.get("pr") {
                Some(val) => match tera::from_value::<Vec<tera::Value>>(val.clone()) {
                    Ok(val) => val,
                    Err(_) => return Err(
                        "argument 'pr' must be a a list of numbers (optionally prefixed with #)"
                            .into(),
                    ),
                },
                None => return Err("required argument 'pr' is missing".into()),
            };

            let mut res = String::new();

            if let [first, rem @ ..] = &*prs {
                let add_pr = |res: &mut String, val| -> tera::Result<()> {
                    let pr = extract_pr(val)?;
                    let link = md_pr_link(pr, &repo);
                    res.push_str(&link);
                    Ok(())
                };

                res.push('(');
                add_pr(&mut res, first)?;
                for pr in rem {
                    res.push_str(", ");
                    add_pr(&mut res, pr)?;
                }
                res.push(')');
            }

            Ok(tera::to_value(res)?)
        },
    )
}

pub fn make_pr_md_link(repo: String) -> impl tera::Function {
    Box::new(
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let repo = extract_repo(&args, &repo)?;
            let pr = extract_pr_from_args(args)?;
            let url = md_pr_link(pr, &repo);
            Ok(tera::to_value(url)?)
        },
    )
}

fn extract_pr(val: &tera::Value) -> tera::Result<u64> {
    match tera::from_value::<u64>(val.clone()).ok().or_else(|| {
        let val = tera::from_value::<String>(val.clone()).ok()?;
        val.parse()
            .ok()
            .or_else(|| val.strip_prefix('#')?.parse().ok())
    }) {
        Some(val) => Ok(val),
        None => return Err("argument 'pr' must be a number (optionally prefixed with #)".into()),
    }
}

fn extract_pr_from_args(args: &HashMap<String, tera::Value>) -> tera::Result<u64> {
    match args.get("pr") {
        Some(val) => extract_pr(val),
        None => return Err("required argument 'pr' is missing".into()),
    }
}

fn extract_repo(args: &HashMap<String, tera::Value>, default: &str) -> tera::Result<String> {
    match args.get("repo") {
        Some(val) => {
            tera::from_value(val.clone()).map_err(|_| "argument 'repo' must be a string".into())
        }
        None => Ok(default.to_owned()),
    }
}

fn pr_url(pr: u64, repo: &str) -> String {
    format!("https://github.com/{repo}/pull/{pr}")
}

fn md_pr_link(pr: u64, repo: &str) -> String {
    format!("[#{pr}]({})", pr_url(pr, repo))
}
