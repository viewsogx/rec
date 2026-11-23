use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

pub struct Command {
    pub name: &'static str,
    pub short: &'static str,
    pub description: &'static str,
}

#[derive(Deserialize, Debug, Serialize, Default)]
pub struct Project {
    pub ignore: Vec<String>,
    pub name: String,
    pub remote: String,
    pub branch: String,
    pub id: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub config: Project,
}

#[derive(Deserialize, Serialize, FromRow, Debug)]
pub struct SavedFile {
    pub id: String,
    pub name: String,
    pub hash: String,
    pub size: String,
    pub location: Option<String>,
    pub cdata: Option<Vec<u8>>,
}

#[derive(FromRow, Debug)]
pub struct Save {
    pub id: String,
    pub timestamp: String,
    pub branch: String,
    pub comment: String,
}

pub const ALL_COMMANDS: &[Command] = &[
    Command {
        name: "init",
        short: "i",
        description: "Initializes a rec project in the current working directory",
    },
    Command {
        name: "status",
        short: "ss",
        description: "Displays the current repository status and modified files",
    },
    Command {
        name: "save",
        short: "s",
        description: "Saves the current changes as a new commit",
    },
    Command {
        name: "log",
        short: "l",
        description: "Displays the commit history of the repository",
    },
    Command {
        name: "diff",
        short: "d",
        description: "Shows the differences between working files and saved changes",
    },
    Command {
        name: "branch",
        short: "b",
        description: "Lists, creates, or switches between repository branches",
    },
    Command {
        name: "load",
        short: "ld",
        description: "Restores the working directory to a previously saved commit",
    },
    Command {
        name: "merge",
        short: "m",
        description: "Merges changes from another branch into the current branch",
    },
    Command {
        name: "login",
        short: "lg",
        description: "Authenticates the user with the remote repository service",
    },
    Command {
        name: "push",
        short: "p",
        description: "Uploads local commits to the remote repository",
    },
    Command {
        name: "sync",
        short: "sy",
        description: "Clones a remote repository or pulls the latest changes into the local repository",
    },
];
