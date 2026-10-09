# Java documentation evidence correction — 2026-10-09

Base: PR31 `958a106636ad8430e40e45075e49c6eea1496629`. The previous eight-test result did not establish native acceptance: seven tests conditionally read a nonexistent top-level findings array, and the Maven test checked only report_type. Requiring native findings first produced seven failures.

The replacement explicitly invokes JDK 21 through `--java-home` and a Java file. All eight tests passed with OpenJDK 21.0.12.1 (Debian official jdk-headless/jre-headless packages, SHA-256 checked against its package index): complete documented class/constructor/field/method; missing type, method, field, parameter and return documentation; bare tag descriptions; line comments. Each test requires a native finding array, exact rule IDs, tool digest and honest local status. Native tests require an explicit tool and are marked ignored in the portable suite; missing native evidence is never a pass.

```sh
CODEGUARD_TEST_JAVA_HOME=/absolute/jdk21 cargo test -p codeguard-cli \
  --test java_documentation_acceptance -- --ignored
CODEGUARD_TEST_JAVA_HOME=/absolute/jdk21 cargo test -p codeguard-cli \
  --test java_comments_cli actual_jdk -- --ignored
CODEGUARD_TEST_JAVA_HOME=/absolute/jdk21 cargo test -p codeguard-cli \
  --test java_capability_closed_loop -- --ignored
```

The five existing actual-JDK tests also passed: missing→documented; configured project remains open after clean observation; explicit-file mode survives added pom; original-task verify/configuration changes; detailed descriptions with stable tasks and rechecks. The new lifecycle test passed actual bad→fixed→bad observations on one stable task with persisted verification events. Fixed code yields `candidate_absent_unverified_policy`; the task remains open. It does not claim authoritative close/reopen.

These file/JDK probes do not execute Maven or Gradle Javadoc plugins, establish complete project policy, prove installed host feedback, or grant platform/release qualification. Existing plugin-specific evidence is separate. Section15.3 and15.6 remain open. CLI gate tests now require `incomplete`, exit3 and the unresolved authority/obligation diagnostics for unqualified clean and invalid Java projects; they do not claim project allow/deny.
