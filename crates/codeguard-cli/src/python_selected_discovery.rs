//! 指定 Python 文件只观察源码路径和祖先 Ruff 配置；不枚举目录或读取旁支清单。

use crate::discovery::{
    DiscoveryReport, empty_report, inspect_python_ruff, observe_file,
    record_unavailable_ruff_config,
};
use codeguard_adapters::LegacyRegistry;
use codeguard_core::{ObservationPort, ObservedPathKind};
use std::{
    collections::BTreeSet,
    io,
    path::{Component, Path, PathBuf},
    time::Instant,
};

const CONFIG_NAMES: [&str; 4] = [
    ".ruff.toml",
    "ruff.toml",
    "pyproject.toml",
    ".pre-commit-config.yaml",
];

/// 每次文件观察前后复核共同截止时间；禁止通过该端口枚举任意目录。
struct SelectedObservation<'a, P> {
    inner: &'a P,
    deadline: Instant,
}

impl<P: ObservationPort> SelectedObservation<'_, P> {
    fn check_deadline(&self) -> io::Result<()> {
        if Instant::now() >= self.deadline || codeguard_runtime::sigint_cancellation_requested() {
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "selected discovery budget exhausted",
            ))
        } else {
            Ok(())
        }
    }
}

impl<P: ObservationPort> ObservationPort for SelectedObservation<'_, P> {
    fn classify(&self, path: &Path) -> io::Result<ObservedPathKind> {
        self.check_deadline()?;
        let kind = self.inner.classify(path)?;
        self.check_deadline()?;
        Ok(kind)
    }
    fn children(&self, _directory: &Path) -> io::Result<Vec<PathBuf>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "selected discovery never enumerates directories",
        ))
    }
    fn read_bounded(&self, path: &Path, max_bytes: u64) -> io::Result<Vec<u8>> {
        self.check_deadline()?;
        let bytes = self.inner.read_bounded(path, max_bytes)?;
        self.check_deadline()?;
        Ok(bytes)
    }
}

/// 观察所选普通 Python 源码及祖先配置，沿用既有 Ruff 配置优先级和点目录策略。
/// 参数为规范根、已受限相对路径、注册表、观察端口和共同截止时间；结果不描述项目其它范围。
pub(crate) fn discover_selected<P: ObservationPort>(
    root: &Path,
    selected: &[String],
    registry: &LegacyRegistry,
    observation: &P,
    deadline: Instant,
) -> DiscoveryReport {
    let observation = SelectedObservation {
        inner: observation,
        deadline,
    };
    let mut report = empty_report(root);
    let mut directories = BTreeSet::new();
    if !matches!(observation.classify(root), Ok(ObservedPathKind::Directory)) {
        report.observation_complete = false;
        report.blocked_paths.push(".".into());
    } else {
        for relative in selected {
            if !safe_relative(relative) {
                report.observation_complete = false;
                report.blocked_paths.push(relative.clone());
                continue;
            }
            let mut path = root.to_path_buf();
            let mut ancestors = Vec::new();
            let mut valid = true;
            for component in Path::new(relative).components() {
                ancestors.push(path.clone());
                path.push(component);
                let last = path == root.join(relative);
                if !matches!(
                    (last, observation.classify(&path)),
                    (false, Ok(ObservedPathKind::Directory)) | (true, Ok(ObservedPathKind::File))
                ) {
                    valid = false;
                    report.observation_complete = false;
                    report.blocked_paths.push(relative.clone());
                    break;
                }
            }
            if valid {
                directories.extend(ancestors);
                report.observed_entries += 1;
                observe_file(
                    &mut report,
                    registry,
                    &observation,
                    &path,
                    relative,
                    &BTreeSet::new(),
                );
            }
        }
        'configs: for directory in directories {
            for name in CONFIG_NAMES {
                if observation.check_deadline().is_err() {
                    break 'configs;
                }
                let candidate = directory.join(name);
                match observation.classify(&candidate) {
                    Ok(ObservedPathKind::File) => {
                        let relative = candidate
                            .strip_prefix(root)
                            .expect("bounded ancestor")
                            .to_string_lossy()
                            .replace(std::path::MAIN_SEPARATOR, "/");
                        report.observed_entries += 1;
                        observe_file(
                            &mut report,
                            registry,
                            &observation,
                            &candidate,
                            &relative,
                            &BTreeSet::new(),
                        );
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    _ => {
                        let relative = candidate
                            .strip_prefix(root)
                            .expect("bounded ancestor")
                            .to_string_lossy()
                            .replace(std::path::MAIN_SEPARATOR, "/");
                        report.observation_complete = false;
                        record_unavailable_ruff_config(&mut report, &relative);
                        report.blocked_paths.push(relative);
                    }
                }
            }
        }
        inspect_python_ruff(root, &observation, &mut report);
    }
    if Instant::now() >= deadline {
        report.observation_complete = false;
        report
            .unknown_conditions
            .push("selected_discovery_deadline_exceeded".into());
    }
    if codeguard_runtime::sigint_cancellation_requested() {
        report.observation_complete = false;
        report.unknown_conditions.push("request_cancelled".into());
    }
    report.blocked_paths.sort();
    report.blocked_paths.dedup();
    report
}

fn safe_relative(relative: &str) -> bool {
    !relative.is_empty()
        && !relative.contains('\\')
        && !relative.chars().any(char::is_control)
        && Path::new(relative)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && !relative
            .split('/')
            .any(|part| part.is_empty() || part.starts_with('.') || part == "node_modules")
}

#[cfg(test)]
mod tests {
    use super::discover_selected;
    use codeguard_adapters::legacy_registry;
    use codeguard_core::{ObservationPort, ObservedPathKind};
    use std::{
        cell::RefCell,
        collections::BTreeMap,
        io,
        path::{Path, PathBuf},
        time::{Duration, Instant},
    };

    #[derive(Default)]
    struct Observation {
        kinds: BTreeMap<PathBuf, ObservedPathKind>,
        contents: BTreeMap<PathBuf, Vec<u8>>,
        classified: RefCell<Vec<PathBuf>>,
        reads: RefCell<Vec<PathBuf>>,
    }
    impl Observation {
        fn project() -> Self {
            let mut o = Self::default();
            for dir in ["/project", "/project/src"] {
                o.kinds.insert(dir.into(), ObservedPathKind::Directory);
            }
            o.kinds
                .insert("/project/src/app.py".into(), ObservedPathKind::File);
            o.config("/project/ruff.toml", "[lint]\nselect = ['F401']\n");
            o
        }
        fn config(&mut self, path: &str, content: &str) {
            self.kinds.insert(path.into(), ObservedPathKind::File);
            self.contents
                .insert(path.into(), content.as_bytes().to_vec());
        }
    }
    impl ObservationPort for Observation {
        fn classify(&self, path: &Path) -> io::Result<ObservedPathKind> {
            self.classified.borrow_mut().push(path.into());
            self.kinds
                .get(path)
                .copied()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        }
        fn children(&self, _path: &Path) -> io::Result<Vec<PathBuf>> {
            panic!("selected discovery must never enumerate a directory")
        }
        fn read_bounded(&self, path: &Path, _max: u64) -> io::Result<Vec<u8>> {
            self.reads.borrow_mut().push(path.into());
            self.contents
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::from(io::ErrorKind::PermissionDenied))
        }
    }
    fn observe(o: &Observation) -> crate::discovery::DiscoveryReport {
        discover_selected(
            Path::new("/project"),
            &["src/app.py".into()],
            &legacy_registry().unwrap(),
            o,
            Instant::now() + Duration::from_secs(5),
        )
    }

    #[test]
    fn selected_discovery_never_enumerates_or_observes_unrelated_files() {
        let mut o = Observation::project();
        o.config("/project/unrelated/ruff.toml", "bad");
        o.config(
            "/project/src/pyproject.toml",
            "[project]\nname = 'example'\n",
        );
        let r = observe(&o);
        assert!(r.observation_complete);
        assert_eq!(r.languages["python"].source_files.len(), 1);
        assert_eq!(r.observed_entries, 3);
        assert!(
            o.classified
                .borrow()
                .iter()
                .all(|p| !p.to_string_lossy().contains("unrelated"))
        );
        assert_eq!(
            r.checker_configurations[0].configuration_ref.as_str(),
            "ruff.toml"
        );
    }

    #[test]
    fn nearest_linked_config_is_unknown_instead_of_falling_back_to_parent() {
        let mut o = Observation::project();
        o.kinds
            .insert("/project/src/ruff.toml".into(), ObservedPathKind::Symlink);
        let r = observe(&o);
        assert!(!r.observation_complete);
        assert_eq!(r.checker_configurations[0].configuration, "unknown");
        assert_eq!(
            r.checker_configurations[0].configuration_ref.as_str(),
            "src/ruff.toml"
        );
    }

    #[test]
    fn nearest_dot_ruff_config_preserves_original_priority() {
        let mut o = Observation::project();
        o.config("/project/src/.ruff.toml", "[lint]\nselect = ['E501']\n");
        o.config("/project/src/ruff.toml", "[lint]\nselect = ['F401']\n");
        let r = observe(&o);
        assert!(r.observation_complete);
        assert_eq!(
            r.checker_configurations[0].configuration_ref.as_str(),
            "src/.ruff.toml"
        );
    }

    #[test]
    fn symlink_ancestor_cannot_read_config_or_source_outside_the_workspace() {
        let mut o = Observation::project();
        o.kinds
            .insert("/project/src".into(), ObservedPathKind::Symlink);
        let r = observe(&o);
        assert!(!r.observation_complete);
        assert!(r.languages.is_empty());
        assert!(o.reads.borrow().is_empty());
    }

    #[test]
    fn expired_deadline_does_not_even_classify_the_root() {
        let o = Observation::project();
        let r = discover_selected(
            Path::new("/project"),
            &["src/app.py".into()],
            &legacy_registry().unwrap(),
            &o,
            Instant::now() - Duration::from_millis(1),
        );
        assert!(!r.observation_complete);
        assert!(
            r.unknown_conditions
                .contains(&"selected_discovery_deadline_exceeded".to_owned())
        );
        assert!(o.classified.borrow().is_empty());
    }
}
