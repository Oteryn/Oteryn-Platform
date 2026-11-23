use oteryn_admin::rbac::{Error, staff::operator_grant};
use std::process::ExitCode;

const USAGE: &str = "Usage: oteryn-portal admin-cli staff grant-role --email <email> --role <role>";

fn grant_args(args: &[String]) -> Option<(&str, &str)> {
    let [area, command, first, first_value, second, second_value] = args else {
        return None;
    };
    if area != "staff" || command != "grant-role" {
        return None;
    }
    match (first.as_str(), second.as_str()) {
        ("--email", "--role") => Some((first_value, second_value)),
        ("--role", "--email") => Some((second_value, first_value)),
        _ => None,
    }
}

pub async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(2).collect();
    if args.as_slice() == ["--help"] {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let Some((email, role)) = grant_args(&args) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let Ok(config) = oteryn_core::Config::from_env() else {
        eprintln!("Portal configuration unavailable");
        return ExitCode::FAILURE;
    };
    let Ok(pool) = oteryn_db::lazy_pool(config.database_url()) else {
        eprintln!("Portal database unavailable");
        return ExitCode::FAILURE;
    };
    let result = operator_grant(&pool, email, role).await;
    pool.close().await;
    match result {
        Ok(granted) => {
            println!(
                "{}",
                if granted {
                    "Role granted"
                } else {
                    "Role already assigned"
                }
            );
            ExitCode::SUCCESS
        }
        Err(Error::Storage(_)) => {
            eprintln!("Portal database unavailable");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_flag_orders_and_requires_each_flag_once() {
        let parse = |tail: [&str; 4]| {
            let args = ["staff", "grant-role"]
                .into_iter()
                .chain(tail)
                .map(String::from)
                .collect::<Vec<_>>();
            grant_args(&args).map(|(email, role)| (email.to_owned(), role.to_owned()))
        };
        let expected = Some(("staff@example.invalid".into(), "support".into()));
        assert_eq!(
            parse(["--email", "staff@example.invalid", "--role", "support"]),
            expected
        );
        assert_eq!(
            parse(["--role", "support", "--email", "staff@example.invalid"]),
            expected
        );
        assert_eq!(parse(["--role", "support", "--role", "moderator"]), None);
        assert_eq!(grant_args(&["staff".into(), "grant-role".into()]), None);
    }
}
