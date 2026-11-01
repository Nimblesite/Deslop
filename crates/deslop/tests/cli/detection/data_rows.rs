//! [CLONE-NOISE-DATA-ROWS] Table rows and import lists are data, not copies.

use super::*;

const MIN_NODES: &str = "30";
const NO_CLUSTERS: usize = 0;
const EDITED_TABLE_SPANS: [(u64, u64); 2] = [(319, 404), (405, 530)];
const GO_LEFT_TABLE: &str = r#"package left

var rows = []row{
	{"slice[1]", "{{index .SI 1}}", "4", tVal, true},
	{"slice[HUGE]", "{{index .SI 10}}", "", tVal, false},
	{"slice[WRONG]", "{{index .SI `hello`}}", "", tVal, false},
	{"slice[nil]", "{{index .SI nil}}", "", tVal, false},
}
"#;
const GO_RIGHT_TABLE: &str = r#"package right

var rows = []row{
	{"bug13", "{{print (.Copy).I}}", "17", tVal, true},
	{"bug14a", "{{(nil).True}}", "", tVal, false},
	{"bug14b", "{{$x := nil}}{{$x.anything}}", "", tVal, false},
	{"bug14c", `{{$x := (1.0)}}{{$x.anything}}`, "", tVal, false},
}
"#;
const GO_LEFT_MAPPINGS: &str = r#"package left

func init() {
	ns.AddMethodMapping(ctx.Where,
		[]string{"where"},
		[][2]string{},
	)
	ns.AddMethodMapping(ctx.Append,
		[]string{"append"},
		[][2]string{},
	)
	ns.AddMethodMapping(ctx.Group,
		nil,
		[][2]string{{`{{ group 1 }}`, `1`}},
	)
}
"#;
const GO_RIGHT_MAPPINGS: &str = r#"package right

func init() {
	ns.AddMethodMapping(ctx.RelURL,
		[]string{"relURL"},
		[][2]string{},
	)
	ns.AddMethodMapping(ctx.RelLangURL,
		nil,
		[][2]string{{`{{ relLangURL 2 }}`, `2`}},
	)
	ns.AddMethodMapping(ctx.RelRef,
		[]string{"relref"},
		[][2]string{},
	)
}
"#;
const JS_OBJECT_ROWS: &str = r"const attributes = [
  {
    name: 'k1',
    read: getSVGProperty('k1'),
    containerTagName: 'svg',
    tagName: 'feComposite',
  },
  {
    name: 'k2',
    read: getSVGProperty('k2'),
    containerTagName: 'svg',
    tagName: 'feComposite',
  },
  { name: 'unrelated', read: getAttribute('unrelated') },
  { name: 'filler', overloadedBoolean: true },
  {
    name: 'targetX',
    read: getSVGProperty('targetX'),
    containerTagName: 'svg',
    tagName: 'feConvolveMatrix',
  },
  {
    name: 'targetY',
    read: getSVGProperty('targetY'),
    containerTagName: 'svg',
    tagName: 'feConvolveMatrix',
  },
];
";
const FS_INLINE_ROWS: &str = r#"module Tests

[<Theory>]
[<InlineData("00000000-0000-0000-0000-000000000000", "Success: 0")>]
[<InlineData("00000000-0000-0000-0000-00000000000", "Not Found")>]
[<InlineData("0000000000000000000000", "Success: d3344dd3")>]
[<InlineData("DBvYFN7y#$@u933Kc8pM#^", "Not Found")>]
[<InlineData("8b3557db-fa-c0c90785ec0b", "Not Found")>]
let ``guid route`` (url: string, expected: string) = check url expected

[<Theory>]
[<InlineData("/pet/0", "PetId: 0")>]
[<InlineData("/pet/123", "PetId: 123")>]
[<InlineData("/pet/-1", "PetId: -1")>]
[<InlineData("/pet/abc", "Not Found")>]
[<InlineData("/pet/123abc", "Not Found")>]
let ``pet route`` (url: string, expected: string) = verify url expected
"#;
const DART_LEFT_EXPECTS: &str = r"void main() {
  test('headers', () async {
    final content = await read();
    for (final line in content.split('\n')) {
      log(line.trim());
    }
    if (content.isEmpty) {
      fail('empty');
    }
    expect(content, contains('Stringkey: 1'));
    expect(content, contains('Numkey: 2'));
    expect(content, contains('Booleankey: false'));
  });
}
";
const DART_RIGHT_EXPECTS: &str = r"void main() {
  test('preflight', () async {
    final reason = await explain();
    expect(reason, startsWith(baseReason));
    expect(reason, contains('cross-origin'));
    expect(reason, contains('PUT'));
    expect(reason, contains('CORS preflight'));
  });
}
";
const PHP_LICENCE: &str =
    "<?php declare(strict_types=1);\n\n/*\n * This file is part of Example.\n */\n\n";
const PHP_LEFT_IMPORTS: &str = "namespace Example\\Test\\Resolver;\n\nuse Example\\Resolver\\Reason;\nuse Example\\Resolver\\Pool;\nuse Example\\Package\\Link;\nuse Example\\Semver\\Constraint;\nuse Example\\Test\\TestCase;\n\n";
const PHP_RIGHT_IMPORTS: &str = "namespace Example\\Test\\Policy;\n\nuse Example\\Policy\\IgnoreRule;\nuse Example\\Policy\\ListConfig;\nuse Example\\Policy\\MalwareConfig;\nuse Example\\Semver\\MatchAll;\nuse Example\\Semver\\Parser;\n\n";
const PHP_LEFT_CLASS: &str = "class LeftTest\n{\n    public function title(): string\n    {\n        return 'left';\n    }\n}\n";
const PHP_RIGHT_CLASS: &str =
    "class RightTest\n{\n    public function size(): int\n    {\n        return 2;\n    }\n}\n";
const JS_COPIED_TABLE: &str = r"export const routes = [
  { path: '/users', method: 'GET', auth: true, handler: 'listUsers' },
  { path: '/users/:id', method: 'GET', auth: true, handler: 'showUser' },
  { path: '/users/:id', method: 'PUT', auth: true, handler: 'saveUser' },
  { path: '/users/:id', method: 'DELETE', auth: true, handler: 'dropUser' },
  { path: '/teams', method: 'GET', auth: true, handler: 'listTeams' },
  { path: '/teams/:id', method: 'GET', auth: true, handler: 'showTeam' },
  { path: '/teams/:id', method: 'PUT', auth: true, handler: 'saveTeam' },
  { path: '/teams/:id', method: 'DELETE', auth: true, handler: 'dropTeam' },
];
";
const JS_LEFT_TESTS: &str = r"describe('left', () => {
  it('accepts ada', () => {
    const user = lookup('ada');
    if (!user.flags.includes('active')) {
      audit.record('rejected', user.id);
      throw new Error('inactive user');
    }
    expect(user.role).toBe('member');
    expect(audit.entries()).toHaveLength(0);
  });
  it('accepts grace', () => {
    const user = lookup('grace');
    if (!user.flags.includes('active')) {
      audit.record('rejected', user.id);
      throw new Error('inactive user');
    }
    expect(user.role).toBe('member');
    expect(audit.entries()).toHaveLength(0);
  });
});
";
const JS_RIGHT_TESTS: &str = r"describe('right', () => {
  it('accepts edsger', () => {
    const user = lookup('edsger');
    if (!user.flags.includes('active')) {
      audit.record('rejected', user.id);
      throw new Error('inactive user');
    }
    expect(user.role).toBe('member');
    expect(audit.entries()).toHaveLength(0);
  });
  it('accepts barbara', () => {
    const user = lookup('barbara');
    if (!user.flags.includes('active')) {
      audit.record('rejected', user.id);
      throw new Error('inactive user');
    }
    expect(user.role).toBe('member');
    expect(audit.entries()).toHaveLength(0);
  });
});
";
const PHP_LEFT_EVENT: &str = r"<?php
class InstallerEvent extends Event
{
    private $composer;
    private $io;
    private $devMode;
    private $transaction;

    public function __construct(string $name, $composer, $io, $devMode, $transaction)
    {
        parent::__construct($name);
        $this->composer = $composer;
        $this->io = $io;
        $this->devMode = $devMode;
        $this->transaction = $transaction;
    }

    public function getComposer()
    {
        return $this->composer;
    }
}
";
const PHP_RIGHT_EVENT: &str = r"<?php
class DownloadEvent extends Event
{
    private $downloader;
    private $url;
    private $type;
    private $context;

    public function __construct(string $name, $downloader, $url, $type, $context)
    {
        parent::__construct($name);
        $this->downloader = $downloader;
        $this->url = $url;
        $this->type = $type;
        $this->context = $context;
    }

    public function getDownloader()
    {
        return $this->downloader;
    }
}
";
const PHP_LEFT_LOGIC: &str = r"<?php
class LeftOrder
{
    public function __construct(string $name, $price, $tax, $discount, $ceiling)
    {
        parent::__construct($name);
        $this->price = $price * $tax;
        $this->discount = max($discount, 0);
        if ($this->price > $ceiling) {
            $this->price = $ceiling;
        }
    }
}
";
const PHP_RIGHT_LOGIC: &str = r"<?php
class RightInvoice
{
    public function __construct(string $label, $amount, $rate, $rebate, $limit)
    {
        parent::__construct($label);
        $this->amount = $amount * $rate;
        $this->rebate = max($rebate, 0);
        if ($this->amount > $limit) {
            $this->amount = $limit;
        }
    }
}
";
const GO_LEFT_FIXTURE: &str = "package left\n\nfunc TestPagination(t *testing.T) {\n\tfiles := `\n-- hugo.toml --\ndisableKinds = [\"taxonomy\"]\n-- content/p1.md --\ntitle: p1\n`\n\n\tb := hugolib.Test(t, files)\n\n\tb.AssertFileExists(\"public/page/1/index.html\", false)\n\tb.AssertFileContent(\"public/page/2/index.html\", \"pagination-default\")\n}\n";
const GO_RIGHT_FIXTURE: &str = "package right\n\nfunc TestPortableText(t *testing.T) {\n\tfiles := `\n-- hugo.toml --\n-- assets/sample.json --\n[{\"_key\": \"a\", \"style\": \"h2\"}]\n`\n\tb := hugolib.Test(t, files)\n\n\tb.AssertFileContent(\"public/index.html\", \"Markdown: ## Heading 2\")\n}\n";
const GO_LEFT_LOGIC_TEST: &str = "package left\n\nfunc TestLeft(t *testing.T) {\n\tfiles := \"left fixture\"\n\tb := hugolib.Test(t, files)\n\tfor _, name := range b.Pages() {\n\t\tif name == \"\" {\n\t\t\tt.Fatal(\"empty left\")\n\t\t}\n\t}\n}\n";
const GO_RIGHT_LOGIC_TEST: &str = "package right\n\nfunc TestRight(t *testing.T) {\n\tfiles := \"right fixture\"\n\tb := hugolib.Test(t, files)\n\tfor _, name := range b.Pages() {\n\t\tif name == \"\" {\n\t\t\tt.Fatal(\"empty right\")\n\t\t}\n\t}\n}\n";
const GO_LOGIC_LEFT: &str = r"package left

func apply(order Order) int {
	total := order.Base
	total = adjust(total, order.Tax)
	total = adjust(total, order.Fee)
	if total > order.Ceiling {
		total = order.Ceiling
	}
	return total
}
";
const GO_LOGIC_RIGHT: &str = r"package right

func charge(invoice Invoice) int {
	amount := invoice.Base
	amount = adjust(amount, invoice.Tax)
	amount = adjust(amount, invoice.Fee)
	if amount > invoice.Ceiling {
		amount = invoice.Ceiling
	}
	return amount
}
";

/// Scans `files` and returns the report.
fn scan(dir: &str, files: &[(&str, &str)]) -> Result<serde_json::Value> {
    let (_tmp, root) = crate::common::scan_dir::temp_scan_dir(dir)?;
    for (name, source) in files {
        fs::write(root.join(name), source)?;
    }
    crate::common::scan_dir::run_report_min_nodes(&root, MIN_NODES)
}

/// Asserts a rows-only corpus reports nothing at all.
fn assert_rows_are_not_clones(label: &str, files: &[(&str, &str)]) -> Result<()> {
    let report = scan(label, files)?;
    let clusters = array_or_empty(&report, "/clusters");
    assert_eq!(clusters.len(), NO_CLUSTERS, "{label}: {clusters:#?}");
    Ok(())
}

/// Asserts a real clone's two files are both reported by one cluster.
fn assert_cluster_spans(
    label: &str,
    files: &[(&str, &str)],
    left: &str,
    right: &str,
) -> Result<()> {
    let report = scan(label, files)?;
    let clusters = array_or_empty(&report, "/clusters");
    let spanning = clusters.iter().any(|cluster| {
        let paths = cluster_file_paths(cluster);
        paths.iter().any(|path| path.ends_with(left))
            && paths.iter().any(|path| path.ends_with(right))
    });
    assert!(
        spanning,
        "{label}: expected {left} and {right} in one cluster: {clusters:#?}"
    );
    Ok(())
}

#[test]
fn go_test_tables_with_different_rows_are_not_clones() -> Result<()> {
    assert_rows_are_not_clones(
        "go-table",
        &[("left.go", GO_LEFT_TABLE), ("right.go", GO_RIGHT_TABLE)],
    )
}

#[test]
fn go_registration_rows_are_not_clones() -> Result<()> {
    assert_rows_are_not_clones(
        "go-mappings",
        &[
            ("left.go", GO_LEFT_MAPPINGS),
            ("right.go", GO_RIGHT_MAPPINGS),
        ],
    )
}

#[test]
fn javascript_object_rows_of_one_array_are_not_clones() -> Result<()> {
    assert_rows_are_not_clones("js-rows", &[("attributes.js", JS_OBJECT_ROWS)])
}

#[test]
fn fsharp_inline_data_rows_are_not_clones() -> Result<()> {
    assert_rows_are_not_clones("fs-rows", &[("Tests.fs", FS_INLINE_ROWS)])
}

#[test]
fn dart_expectation_rows_of_different_lengths_are_not_clones() -> Result<()> {
    assert_rows_are_not_clones(
        "dart-expects",
        &[
            ("left.dart", DART_LEFT_EXPECTS),
            ("right.dart", DART_RIGHT_EXPECTS),
        ],
    )
}

#[test]
fn php_import_lists_are_not_clones() -> Result<()> {
    let left = format!("{PHP_LICENCE}{PHP_LEFT_IMPORTS}{PHP_LEFT_CLASS}");
    let right = format!("{PHP_LICENCE}{PHP_RIGHT_IMPORTS}{PHP_RIGHT_CLASS}");
    assert_rows_are_not_clones("php-imports", &[("Left.php", &left), ("Right.php", &right)])
}

#[test]
fn a_table_copied_between_files_is_still_a_clone() -> Result<()> {
    assert_cluster_spans(
        "js-copied",
        &[("left.js", JS_COPIED_TABLE), ("right.js", JS_COPIED_TABLE)],
        "left.js",
        "right.js",
    )
}

// The real tests/model_meta/results.py from django, whose tables are copied and then edited.
#[test]
fn a_table_copied_and_then_edited_is_still_a_clone() -> Result<()> {
    let json = run_min_nodes("data-rows-edited-table", MIN_NODES)?;
    let report: serde_json::Value = serde_json::from_str(&json)?;
    let clusters = array_or_empty(&report, "/clusters");
    let spans: Vec<Vec<(u64, u64)>> = clusters.iter().map(occurrence_lines).collect();
    assert!(spans.contains(&EDITED_TABLE_SPANS.to_vec()), "{spans:?}");
    Ok(())
}

/// One cluster's occurrences as start and end lines.
fn occurrence_lines(cluster: &serde_json::Value) -> Vec<(u64, u64)> {
    array_or_empty(cluster, "/occurrences")
        .iter()
        .filter_map(|occurrence| {
            let line = |key: &str| occurrence.get(key).and_then(serde_json::Value::as_u64);
            Some((line("start_line")?, line("end_line")?))
        })
        .collect()
}

#[test]
fn test_cases_holding_logic_are_still_clones() -> Result<()> {
    assert_cluster_spans(
        "js-tests",
        &[("left.js", JS_LEFT_TESTS), ("right.js", JS_RIGHT_TESTS)],
        "left.js",
        "right.js",
    )
}

#[test]
fn renamed_statements_with_a_branch_are_still_clones() -> Result<()> {
    assert_cluster_spans(
        "go-logic",
        &[("left.go", GO_LOGIC_LEFT), ("right.go", GO_LOGIC_RIGHT)],
        "left.go",
        "right.go",
    )
}

#[test]
fn php_value_classes_are_not_clones() -> Result<()> {
    assert_rows_are_not_clones(
        "php-events",
        &[("Left.php", PHP_LEFT_EVENT), ("Right.php", PHP_RIGHT_EVENT)],
    )
}

#[test]
fn php_constructors_holding_logic_are_still_clones() -> Result<()> {
    assert_cluster_spans(
        "php-logic",
        &[("Left.php", PHP_LEFT_LOGIC), ("Right.php", PHP_RIGHT_LOGIC)],
        "Left.php",
        "Right.php",
    )
}

#[test]
fn go_tests_differing_only_in_their_fixture_are_not_clones() -> Result<()> {
    assert_rows_are_not_clones(
        "go-fixture",
        &[("left.go", GO_LEFT_FIXTURE), ("right.go", GO_RIGHT_FIXTURE)],
    )
}

#[test]
fn go_tests_holding_logic_are_still_clones() -> Result<()> {
    assert_cluster_spans(
        "go-test-logic",
        &[
            ("left.go", GO_LEFT_LOGIC_TEST),
            ("right.go", GO_RIGHT_LOGIC_TEST),
        ],
        "left.go",
        "right.go",
    )
}
