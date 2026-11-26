pub struct Command {
    pub name: &'static str,
    pub short: &'static str,
    pub description: &'static str,
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
        name: "list",
        short: "ls",
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
