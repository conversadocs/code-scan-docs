use csd::context::categorize_diff;

#[test]
fn test_feat_detection() {
    let diff = "\
+ fn new_feature() {
+     println!(\"Hello\");
+ }";
    let categories = categorize_diff(diff);
    assert!(categories.contains(&"feat".to_string()));
}

#[test]
fn test_fix_detection() {
    let diff = "\
+ // fix: prevent panic
+ let value = some_option.unwrap();";
    let categories = categorize_diff(diff);
    assert!(categories.contains(&"fix".to_string()));
}

#[test]
fn test_docs_detection() {
    let diff = "\
+ /// Adds two numbers
+ // This is a helper function";
    let categories = categorize_diff(diff);
    assert!(categories.contains(&"docs".to_string()));
}

#[test]
fn test_test_detection() {
    let diff = "\
+ #[test]
+ fn test_example() {
+     assert_eq!(1, 1);
+ }";
    let categories = categorize_diff(diff);
    assert!(categories.contains(&"test".to_string()));
}

#[test]
fn test_style_detection() {
    let diff = "\
+    
+    ";
    let categories = categorize_diff(diff);
    assert!(categories.contains(&"style".to_string()));
}

#[test]
fn test_refactor_detection() {
    let diff = "\
+ let x = some_value;";
    let categories = categorize_diff(diff);
    assert!(categories.contains(&"refactor".to_string()));
}

#[test]
fn test_fallback_to_chore() {
    let diff = "\
+ echo 'Hello, world!'";
    let categories = categorize_diff(diff);
    assert!(categories.contains(&"chore".to_string()));
}
