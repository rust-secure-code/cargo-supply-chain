use crate::common::*;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Author {
    Local { name: String },
    Foreign { name: String },
}

pub fn authors_of(deps: &[SourcedPackage]) -> impl Iterator<Item = Author> + '_ {
    struct AuthorIter<'deps> {
        dependencies: &'deps [SourcedPackage],
        local_todo: Vec<String>,
        foreign_todo: Vec<String>,
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

    AuthorIter {
        dependencies: deps,
        local_todo: vec![],
        foreign_todo: vec![],
    }
}

impl std::fmt::Display for Author {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Author::Local { name } => write!(f, "{}\t\tlocal", name),
            Author::Foreign { name } => write!(f, "{}\t\tunknown registry", name),
        }
    }
}
