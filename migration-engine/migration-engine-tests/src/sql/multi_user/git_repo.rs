use anyhow::Context;
use std::path::{Path, PathBuf};

pub(crate) struct GitRepo {
    pub(crate) name: &'static str,
    pub(crate) root_dir: PathBuf,
    pub(crate) repo: git2::Repository,
}

impl GitRepo {
    pub(crate) fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Returns whether we can merge the other user.
    pub(crate) fn prepare_merge(&self, other: &GitRepo) -> anyhow::Result<(git2::Commit<'_>, git2::Index)> {
        let mut remote = self.remote(&other.name, other.root_dir.as_os_str().to_str().unwrap())?;
        let mut fo = git2::FetchOptions::new();
        remote.fetch(&["master"], Some(&mut fo), Some("Test suite fetch"))?;

        let reference = self.repo.resolve_reference_from_short_name("FETCH_HEAD")?;
        let fetched_commit = reference.peel_to_commit()?;
        let last_commit = self.find_last_commit().expect("find last commit");
        let merge_options = git2::MergeOptions::new();

        let index = self
            .repo
            .merge_commits(&last_commit, &fetched_commit, Some(&merge_options))
            .with_context(|| format!("Merging {} into {}", &other.name, self.name))?;

        Ok((last_commit, index))
    }

    fn remote(&self, name: &str, url: &str) -> anyhow::Result<git2::Remote<'_>> {
        self.repo
            .find_remote(name)
            .or_else(|_err| self.repo.remote(name, url))
            .map_err(Into::into)
    }

    fn git_commit_author(&self) -> anyhow::Result<git2::Signature<'_>> {
        Ok(git2::Signature::now(self.name, self.name)?)
    }

    fn find_last_commit(&self) -> Option<git2::Commit<'_>> {
        self.repo.head().and_then(|r| r.peel_to_commit()).ok()
    }

    pub(crate) fn commit(&self, message: &str) -> anyhow::Result<()> {
        let mut index = self.repo.index()?;
        index.add_all(&["."], git2::IndexAddOption::DEFAULT, None)?;

        let oid = index.write_tree()?;
        let tree = self.repo.find_object(oid, None)?;

        let last_commit = self.find_last_commit();
        let commit_parents: Vec<&git2::Commit<'_>> = if let Some(commit) = last_commit.as_ref() {
            vec![commit]
        } else {
            Vec::new()
        };
        let author = self.git_commit_author()?;
        self.repo.commit(
            Some("HEAD"),
            &author,
            &author,
            message,
            tree.as_tree().unwrap(),
            &commit_parents,
        )?;

        let mut co = git2::build::CheckoutBuilder::new();
        co.allow_conflicts(true);
        self.repo.checkout_index(Some(&mut index), Some(&mut co))?;

        Ok(())
    }

    pub fn merge_from(&self, other: &GitRepo) -> anyhow::Result<()> {
        let (last_commit, mut index) = self.prepare_merge(other)?;
        let oid = index
            .write_tree_to(&self.repo)
            .with_context(|| format!("Writing tree after merging {} into {}", &other.name, self.name))?;

        let tree = self.repo.find_object(oid, None)?;
        let author = self.git_commit_author()?;
        let message = format!("Merge {} into master", other.name);

        self.repo.commit(
            Some("HEAD"),
            &author,
            &author,
            &message,
            tree.as_tree().expect("as tree"),
            &[&last_commit],
        )?;

        let mut co = git2::build::CheckoutBuilder::new();
        co.allow_conflicts(true);
        co.force();
        self.repo.checkout_index(Some(&mut index), Some(&mut co))?;

        Ok(())
    }
}
