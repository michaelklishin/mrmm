extern crate gh;

extern crate clap;

use std::{env, process};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use clap::{Arg, App, ArgMatches, SubCommand};

use gh::milestones;

const TOKEN_ENV_VAR: &str = "GITHUB_TOKEN";

fn main() {
    let cli = App::new("mrmm (Milti-repo Milestone Manager)")
        .about("Manage milestones of a group of repositories at once")
        .author("by Team RabbitMQ")
        // TODO: should this be required instead of an env variable fallback?
        .arg(Arg::with_name("github-token")
              .short("-t")
              .long("github-token")
              .help("GitHub OAuth 2 token to use")
              .takes_value(true))
        .arg(Arg::with_name("repo-list-file")
              .short("-f")
              .long("repo-list-file")
              .help("A file with the list of repositories, one by line")
              .takes_value(true)
              .required(true))
        .subcommand(SubCommand::with_name("close")
                    .about("Closes a milestone")
                    .arg(Arg::with_name("title")
                         .long("--title")
                         .help("milestone title")
                         .takes_value(true)))
        .subcommand(SubCommand::with_name("create")
                    .about("Creates a milestone")
                    .arg(Arg::with_name("title")
                         .long("--title")
                         .help("milestone title")
                         .takes_value(true)))
        .subcommand(SubCommand::with_name("delete")
                    .about("Deletes a milestone")
                    .arg(Arg::with_name("title")
                         .long("--title")
                         .help("milestone title")
                         .takes_value(true)))
        .get_matches();

    let fallback_token: Option<String> =
        env::var(TOKEN_ENV_VAR).ok();

    let token: Option<String> = cli
        .value_of("github-token")
        .map (|val| String::from(val))
        .or(fallback_token);

    match token {
        Some(val) => run_with_token(&cli, &val),
        None      => terminate(1, "please make sure the GITHUB_TOKEN environment variable is set to a valid token value")
    };
}

fn run_with_token(cli: &ArgMatches, token: &str) {
    let path = cli.value_of("repo-list-file")
        .map(|s| Path::new(s) )
        .unwrap();

    let repos = read_repository_list(&path);
    validate_repos(&repos);
    println!("Have {} repositories to work with", repos.len());

    let client = gh::client(token);
    execute_subcommand(cli, &client, &repos);
}

fn execute_subcommand(cli: &ArgMatches, client: &gh::Client, repos: &Vec<String>) {
    if let Some(cmd) = cli.subcommand_matches("create") {
        execute_create(cmd, client, repos);
    }

    if let Some(cmd) = cli.subcommand_matches("close") {
        execute_close(cmd, client, repos);
    }

    if let Some(cmd) = cli.subcommand_matches("delete") {
        execute_delete(cmd, client, repos);
    }

    terminate(1, "No command specified.");
}

fn execute_create(cmd: &ArgMatches, client: &gh::Client, repos: &Vec<String>) {
    let title = cmd.value_of("title").unwrap();

    for name in repos.iter() {
        let pair: Vec<&str> = name.split("/")
            .collect::<Vec<&str>>();

        let org  = pair.first().unwrap();
        let repo = pair.last().unwrap();

        println!("Creating milestone {} in repository {}/{}", title, org, repo);
        let props = milestones::MilestoneProperties {
            title: String::from(title),
            state: Some(milestones::State::Open),
            description: None,
            due_on: None
        };
        let _ = client.create_milestone(org, repo, &props);
    };

    terminate(0, "Done.");
}

fn execute_close(cmd: &ArgMatches, client: &gh::Client, repos: &Vec<String>) {
    let title = cmd.value_of("title").unwrap();

    for name in repos.iter() {
        let pair: Vec<&str> = name.split("/")
            .collect::<Vec<&str>>();

        let org  = pair.first().unwrap();
        let repo = pair.last().unwrap();

        match client.get_milestone_with_title(&org, &repo, &title) {
            Err(e) => println!("Failed to close milestone {} in repository {}/{}: {:?}",
                               title, org, repo, e),
            Ok(_)  => {
                println!("Closing milestone {} in repository {}/{}", title, org, repo);
                let _ = client.close_milestone(org, repo, title);
            }
        }
    };

    terminate(0, "Done.");
}

fn execute_delete(cmd: &ArgMatches, client: &gh::Client, repos: &Vec<String>) {
    let title = cmd.value_of("title").unwrap();

    for name in repos.iter() {
        let pair: Vec<&str> = name.split("/")
            .collect::<Vec<&str>>();

        let org  = pair.first().unwrap();
        let repo = pair.last().unwrap();

        println!("Deleting milestone {} in repository {}/{}", title, org, repo);
        let _ = client.delete_milestone_with_title(org, repo, title);
    };

    terminate(0, "Done.");
}

fn read_repository_list(path: &Path) -> Vec<String> {
    let printable_path = path.display();
    println!("Will load the list of repositories from {}", printable_path);

    let reader = match File::open(&path) {
        Err(reason) => panic!("Aborting. Couldn't open {}: {}", printable_path, reason.to_string()),
        Ok(file)    => BufReader::new(file)
    };

    reader.lines()
        .map(|line| { line.unwrap() })
        .collect()
}

fn validate_repos(repos: &Vec<String>) -> &Vec<String> {
    let non_pairs: Vec<&String> = repos.iter()
        .filter(|s| {
            let xs = s.split("/").collect::<Vec<&str>>();
            xs.len() != 2
        })
        .collect::<Vec<&String>>();

    if non_pairs.len() > 0 {
        panic!("Aborting. Some repositories are not in the org/repo format, e.g. {}", non_pairs.first().unwrap());
    }

    repos
}


fn terminate(code: i32, message: &str) {
    println!("{}", message);

    process::exit(code);
}
