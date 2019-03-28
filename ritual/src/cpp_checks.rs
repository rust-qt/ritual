use ritual_common::target::Target;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct CppCheckerEnv {
    pub target: Target,
    pub cpp_library_version: Option<String>,
}

impl CppCheckerEnv {
    pub fn short_text(&self) -> String {
        format!(
            "{}/{:?}-{:?}-{:?}-{:?}",
            self.cpp_library_version
                .as_ref()
                .map_or("None", |s| s.as_str()),
            self.target.arch,
            self.target.os,
            self.target.family,
            self.target.env
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CppChecksItem {
    pub env: CppCheckerEnv,
    pub is_success: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CppChecks(Vec<CppChecksItem>);

impl CppChecks {
    pub fn has_env(&self, env: &CppCheckerEnv) -> bool {
        self.0.iter().any(|item| &item.env == env)
    }

    pub fn add(&mut self, env: CppCheckerEnv, is_success: bool) {
        self.0.retain(|item| item.env != env);
        self.0.push(CppChecksItem { env, is_success });
    }

    pub fn any_success(&self) -> bool {
        self.0.iter().any(|item| item.is_success)
    }

    pub fn all_success(&self) -> bool {
        self.0.iter().all(|item| item.is_success)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
