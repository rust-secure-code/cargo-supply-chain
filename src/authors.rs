use std::collections::HashSet;
use crates_io_api::{SyncClient, User};
use crate::common::*;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Author {
    CratesUser {
        id: u64,
        login: String,
        name: Option<String>,
        mail: Option<String>,
    },
    Local {
        name: String,
    },
    Foreign {
        name: String,
    },
    CrateError {
        crate_: String,
        version: String,
    },
}

pub fn authors_of(deps: &[SourcedPackage]) -> impl Iterator<Item = Author> + '_ {
    struct AuthorIter<'deps> {
        dependencies: &'deps [SourcedPackage],
        named: HashSet<u64>,
        local_todo: Vec<String>,
        foreign_todo: Vec<String>,
        crates_todo: Vec<User>,
        client: SyncClient,
    }

    impl Iterator for AuthorIter<'_> {
        type Item = Author;
        fn next(&mut self) -> Option<Author> {
            loop {
                if let Some(name) = self.local_todo.pop() {
                    return Some(Author::Local { name });
                }

                if let Some(name) = self.foreign_todo.pop() {
                    return Some(Author::Foreign { name });
                }

                while let Some(user) = self.crates_todo.pop() {
                    if self.named.contains(&user.id) {
                        continue;
                    }

                    self.named.insert(user.id);
                    return Some(Author::CratesUser {
                        id: user.id,
                        login: user.login,
                        name: user.name,
                        mail: user.email,
                    });
                }

                let (first, tail) = self.dependencies.split_first()?;
                self.dependencies = tail;

                match first.source {
                    PkgSource::Local => {
                        self.local_todo = first.package.authors.clone();
                    }
                    PkgSource::CratesIo | PkgSource::Foreign => {
                        self.foreign_todo = first.package.authors.clone();
                    }
                };
            }
        }
    }

    let client = SyncClient::new("cargo-supply-chain", std::time::Duration::from_secs(1)).unwrap();

    AuthorIter {
        dependencies: deps,
        named: HashSet::new(),
        local_todo: vec![],
        foreign_todo: vec![],
        crates_todo: vec![],
        client,
    }
}

impl std::fmt::Display for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Author::CratesUser {
                id: _,
                login,
                name,
                mail,
            } => {
                let display_name = name.as_ref().unwrap_or(&login);
                write!(f, "{}", display_name)?;
                if let Some(mail) = mail {
                    write!(f, "\t{}\tcrates.io", mail)?;
                } else {
                    write!(f, "\t\tcrates.io")?;
                }
                Ok(())
            }
            Author::Local { name } => write!(f, "{}\t\tlocal", name),
            Author::Foreign { name } => write!(f, "{}\t\tunknown registry", name),
            Author::CrateError { crate_, version } => {
                write!(f, "Error resolving crate `{}: {}`", crate_, version)
            }
        }
    }
}
