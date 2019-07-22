use itertools::Itertools;
use ritual_common::target::{Arch, Endian, Env, Family, LibraryTarget, PointerWidth, OS};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CppChecksItem {
    pub env: LibraryTarget,
    pub is_success: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CppChecks(Vec<CppChecksItem>);

impl CppChecks {
    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn has_env(&self, env: &LibraryTarget) -> bool {
        self.0.iter().any(|item| &item.env == env)
    }

    pub fn is_success(&self, env: &LibraryTarget) -> bool {
        self.0
            .iter()
            .find(|item| &item.env == env)
            .map_or(false, |item| item.is_success)
    }

    pub fn has_all_envs(&self, environments: &[LibraryTarget]) -> bool {
        environments.iter().all(|env| self.has_env(env))
    }

    pub fn successful_envs(&self) -> impl Iterator<Item = &LibraryTarget> {
        self.0
            .iter()
            .filter(|item| item.is_success)
            .map(|item| &item.env)
    }

    pub fn add(&mut self, env: LibraryTarget, is_success: bool) {
        self.0.retain(|item| item.env != env);
        self.0.push(CppChecksItem { env, is_success });
    }

    pub fn any_success(&self) -> bool {
        self.0.iter().any(|item| item.is_success)
    }

    pub fn all_success(&self, environments: &[LibraryTarget]) -> bool {
        environments.iter().all(|env| {
            self.0
                .iter()
                .any(|check| &check.env == env && check.is_success)
        })
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn condition(&self, environments: &[LibraryTarget]) -> Condition {
        if !self.any_success() {
            return Condition::False;
        }
        if self.all_success(environments) {
            return Condition::True;
        }
        if self
            .0
            .iter()
            .all(|item| item.env.cpp_library_version.is_some())
        {
            let good_lib_versions = self
                .0
                .iter()
                .filter(|item| item.is_success)
                .map(|item| item.env.cpp_library_version.clone().unwrap())
                .collect_vec();
            if good_lib_versions.len() == 1 {
                let good_version = good_lib_versions.into_iter().next().unwrap();
                return Condition::CppLibraryVersion(good_version);
            } else {
                return Condition::Or(
                    good_lib_versions
                        .into_iter()
                        .map(Condition::CppLibraryVersion)
                        .collect(),
                );
            }
        }

        // TODO: implement more sensible conditions generation
        Condition::True
    }

    pub fn is_always_success_for(&self, other: &CppChecks) -> bool {
        for item in &other.0 {
            if item.is_success && !self.0.iter().any(|i| i.env == item.env && i.is_success) {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condition {
    CppLibraryVersion(String),
    Arch(Arch),
    OS(OS),
    Family(Family),
    Env(Env),
    PointerWidth(PointerWidth),
    Endian(Endian),
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
    True,
    False,
}
