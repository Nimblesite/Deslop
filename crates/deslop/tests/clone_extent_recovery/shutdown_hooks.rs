//! [FUSED-CONTENT-GATE-CALL-TARGET-CONTRACT] A copied lifecycle dispatcher retains its whole module.
//! [FUSED-CONTENT-GATE-CALL-TARGET-PADDING] Changed operations cannot hide behind different surrounding syntax.

use std::{fs, path::Path};

use super::*;

const HOOK_SIDES: [&str; 2] = ["Before.ts", "After.ts"];
const HOOK_TYPES: [&str; 2] = ["BeforeShutdown", "AfterShutdown"];
const HOOK_METHODS: [&str; 2] = ["beforeShutdown", "afterShutdown"];
const WHOLE_MODULE_START: u64 = 1;
const ONE_MODULE_CLONE: usize = 1;
const COMPLETE_DUPLICATION: f64 = 100.0;
const WHOLE_MODULE_END: u64 = 27;
const WHOLE_MODULE_CONTEXT: &str = "one complete hook module";
const METRICS_CONTEXT: &str = "nearly identical metrics";
const SCAN_DIR: &str = "src";
const RANK_FIELD: &str = "rank";
const FILE_COUNT_FIELD: &str = "files_analysed";
const PERCENT_FIELD: &str = "duplication_percent";
const POSITIVE_GUARD: &str = "";
const NEGATED_GUARD: &str = "!";
const GUARDED_CONJUNCTION: &str = "&&";
const GUARDED_DISJUNCTION: &str = "||";
const SHARED_BODY_START: u64 = 16;
const SHARED_BODY_END: u64 = 23;
const INNER_COPY_CONTEXT: &str = "the shared dispatcher body remains a real copy";

macro_rules! hook_module {
    ($contract:expr, $method:expr, $guard:expr, $join:expr) => {
        format!(r"import {{ {contract} }} from './contracts';
import {{ isFunction, isNil, iterate }} from './runtime';
import {{ getNonTransientInstances, getTransientInstances }} from './instances';
import {{ InstanceWrapper, Module }} from './module';
function has{contract}(instance: unknown): instance is {contract} {{
  return isFunction((instance as {contract}).{method});
}}
function callOperator(instances: InstanceWrapper[], signal?: string): Promise<any>[] {{
  return iterate(instances)
    .filter(instance => !isNil(instance))
    .filter(has{contract})
    .map(async instance => (instance as any as {contract}).{method}(signal))
    .toArray();
}}
export async function call{contract}(module: Module, signal?: string): Promise<void> {{
  const providers = module.getNonAliasProviders();
  const [_, moduleClassHost] = providers.shift()!;
  const instances = [...module.controllers, ...providers, ...module.injectables, ...module.middlewares];
  const nonTransientInstances = getNonTransientInstances(instances);
  await Promise.all(callOperator(nonTransientInstances, signal));
  const transientInstances = getTransientInstances(instances);
  await Promise.all(callOperator(transientInstances, signal));
  const moduleClassInstance = moduleClassHost.instance;
  if (moduleClassInstance && {guard}has{contract}(moduleClassInstance) {join} moduleClassHost.isDependencyTreeStatic()) {{
    await moduleClassInstance.{method}(signal);
  }}
}}
", contract = $contract, method = $method, guard = $guard, join = $join)
    };
}

fn write_hook_module(
    root: &Path,
    side: &str,
    names: (&str, &str),
    guard: (&str, &str),
) -> Result<()> {
    let source = hook_module!(names.0, names.1, guard.0, guard.1);
    fs::write(root.join(side), source)?;
    Ok(())
}

fn hook_report(methods: &[&str; 2]) -> Result<Value> {
    configured_hook_report(&HOOK_TYPES, methods, (POSITIVE_GUARD, GUARDED_CONJUNCTION))
}

fn configured_hook_report(
    contracts: &[&str; 2],
    methods: &[&str; 2],
    guard: (&str, &str),
) -> Result<Value> {
    let (_temp, root) = temp_scan_dir(SCAN_DIR)?;
    for ((side, contract), method) in HOOK_SIDES.iter().zip(contracts).zip(methods) {
        write_hook_module(&root, side, (contract, method), guard)?;
    }
    run_report(&root, MIN_NODES)
}

fn assert_whole_hook_module(report: &Value) -> Result<()> {
    let findings = clone_findings(report);
    assert_eq!(findings.len(), ONE_MODULE_CLONE, "{report:#}");
    let cluster = cluster_spanning_sides(report, &HOOK_SIDES, WHOLE_MODULE_CONTEXT)?;
    for side in HOOK_SIDES {
        assert!(
            has_side_span(cluster, side, WHOLE_MODULE_START, WHOLE_MODULE_END),
            "{report:#}"
        );
    }
    assert_eq!(cluster_size(cluster), OCCURRENCE_COUNT);
    assert_eq!(cluster_kind(cluster), NEARLY_IDENTICAL_KIND);
    assert_eq!(field(cluster, RANK_FIELD).as_u64(), Some(FIRST_RANK));
    assert_hook_metrics(report)
}

fn assert_hook_metrics(report: &Value) -> Result<()> {
    assert_eq!(
        field(report, FILE_COUNT_FIELD).as_u64(),
        Some(FILES_ANALYSED)
    );
    let row = per_kind_row(report, NEARLY_IDENTICAL_KIND).context(METRICS_CONTEXT)?;
    assert_eq!(
        field(&row, PERCENT_FIELD).as_f64(),
        Some(COMPLETE_DUPLICATION)
    );
    Ok(())
}

#[test]
fn lifecycle_dispatchers_keep_the_whole_renamed_module() -> Result<()> {
    let control = hook_report(&[HOOK_METHODS[0], HOOK_METHODS[0]])?;
    assert_whole_hook_module(&control)?;
    let renamed = hook_report(&HOOK_METHODS)?;
    assert_whole_hook_module(&renamed)
}

fn assert_only_the_shared_dispatcher_work_is_copied(report: &Value) -> Result<()> {
    let findings = clone_findings(report);
    assert!(!findings.iter().any(complete_hook_module), "{report:#}");
    let body = findings
        .iter()
        .find(|cluster| {
            HOOK_SIDES
                .iter()
                .all(|side| has_side_span(cluster, side, SHARED_BODY_START, SHARED_BODY_END))
        })
        .with_context(|| format!("{INNER_COPY_CONTEXT}: {report:#}"))?;
    assert_eq!(cluster_size(body), OCCURRENCE_COUNT);
    assert_eq!(cluster_kind(body), IDENTICAL_KIND);
    assert_eq!(
        field(report, FILE_COUNT_FIELD).as_u64(),
        Some(FILES_ANALYSED)
    );
    Ok(())
}

fn complete_hook_module(cluster: &Value) -> bool {
    HOOK_SIDES
        .iter()
        .all(|side| has_side_span(cluster, side, WHOLE_MODULE_START, WHOLE_MODULE_END))
}

#[test]
fn changed_operations_on_the_same_contract_keep_only_the_shared_work() -> Result<()> {
    let report = configured_hook_report(
        &[HOOK_TYPES[0]; 2],
        &HOOK_METHODS,
        (POSITIVE_GUARD, GUARDED_CONJUNCTION),
    )?;
    assert_only_the_shared_dispatcher_work_is_copied(&report)
}

#[test]
fn negated_and_optional_type_guards_do_not_rename_the_receiver_contract() -> Result<()> {
    for guard in [
        (NEGATED_GUARD, GUARDED_CONJUNCTION),
        (POSITIVE_GUARD, GUARDED_DISJUNCTION),
    ] {
        let report = configured_hook_report(&HOOK_TYPES, &HOOK_METHODS, guard)?;
        assert_only_the_shared_dispatcher_work_is_copied(&report)?;
    }
    Ok(())
}
