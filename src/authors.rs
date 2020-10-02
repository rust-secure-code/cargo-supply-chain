use std::collections::HashSet;

use cargo_metadata::Dependency;
use crates_io_api::{User, SyncClient};

pub enum Author {
    CratesUser {
        id: u64,
        login: String,
        name: Option<String>,
        mail: Option<String>,
    },
    UnknownSource,
    CrateError {
    },
}

pub fn authors_of(deps: &[Dependency])
    -> impl Iterator<Item=Author> + '_
{
    struct AuthorIter<'deps> {
        dependencies: &'deps [Dependency],
        named: HashSet<u64>,
        todo: Vec<User>,
        client: SyncClient,
    }

    impl Iterator for AuthorIter<'_> {
        type Item = Author;
        fn next(&mut self) -> Option<Author> {
            loop {
                while let Some(user) = self.todo.pop() {
                    if self.named.contains(&user.id) {
                        continue
                    }

                    self.named.insert(user.id);
                    return Some(Author::CratesUser {
                        id: user.id,
                        login: user.login,
                        name: user.name,
                        mail: user.email,
                    })
                }

                let (first, tail) = self.dependencies.split_first()?;
                self.dependencies = tail;

                if let Some(_) = first.registry {
                    return Some(Author::UnknownSource);
                }

                match self.client.crate_authors(&first.name, &format!("{}", first.req)) {
                    Err(_) => return Some(Author::CrateError { }),
                    Ok(authors) => self.todo = authors.users,
                }
            }
        }
    }

    let client = SyncClient::new("cargo-supply-chain", std::time::Duration::from_secs(1))
        .unwrap();

    AuthorIter {
        dependencies: deps,
        named: HashSet::new(),
        todo: vec![],
        client,
    }
}

impl std::fmt::Display for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Author::CratesUser { id: _, login, name, mail } => {
                let display_name = name.as_ref().unwrap_or(&login);
                write!(f, "{}", display_name)?;
                if let Some(mail) = mail {
                    write!(f, "({})", mail)?;
                }
                Ok(())
            },
            Author::UnknownSource => write!(f, "Unknown crate source (private registry?)"),
            Author::CrateError {} => write!(f, "Error resolving crate"),
        }
    }
}
