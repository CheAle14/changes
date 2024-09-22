use std::{path::Path, sync::Arc};

use ignore::{
    gitignore::{Gitignore, Glob},
    Match,
};

pub struct IgnoreChecker {
    git: Option<ignore::gitignore::Gitignore>,
    parent: Option<Arc<Self>>,
}

impl IgnoreChecker {
    pub fn empty() -> Arc<Self> {
        Arc::new(Self {
            git: None,
            parent: None,
        })
    }

    pub fn root<P: AsRef<Path>>(path: P) -> Arc<Self> {
        let (git, _) = Gitignore::new(path);
        Arc::new(Self {
            git: Some(git),
            parent: None,
        })
    }

    pub fn child<P: AsRef<Path>>(self: &Arc<Self>, path: P) -> Arc<Self> {
        let (git, _) = Gitignore::new(path);
        Arc::new(Self {
            git: Some(git),
            parent: Some(Arc::clone(self)),
        })
    }

    fn raw_match(&self, path: &Path, is_dir: bool) -> Match<&Glob> {
        let result = if let Some(git) = self.git.as_ref() {
            git.matched(path, is_dir)
        } else {
            Match::None
        };

        match result {
            Match::None => {
                if let Some(parent) = self.parent.as_ref() {
                    parent.raw_match(path, is_dir)
                } else {
                    Match::None
                }
            }
            m => m,
        }
    }

    pub fn should_ignore<P: AsRef<Path>>(&self, path: P) -> bool {
        let path: &Path = path.as_ref();
        let is_dir = path.is_dir();

        self.raw_match(path, is_dir).is_ignore()
    }
}
