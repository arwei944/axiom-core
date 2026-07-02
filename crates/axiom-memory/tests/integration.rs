use axiom_memory::*;
use serde_json::json;

#[test]
fn test_add_and_get_memory() {
    let mem = WorkingMemory::new(1000);

    let id = mem.add_thought("This is a test thought");
    assert_eq!(mem.item_count(), 1);

    let item = mem.get(&id).unwrap();
    assert_eq!(item.content, "This is a test thought");
    assert_eq!(item.item_type, MemoryItemType::Thought);
}

#[test]
fn test_multiple_item_types() {
    let mem = WorkingMemory::new(1000);

    mem.add_thought("Thinking about problem");
    mem.add_observation("I see something");
    mem.add_action("Do something");
    mem.add_result("Got a result");

    assert_eq!(mem.item_count(), 4);

    let thoughts = mem.filter_by_type(MemoryItemType::Thought);
    assert_eq!(thoughts.len(), 1);

    let observations = mem.filter_by_type(MemoryItemType::Observation);
    assert_eq!(observations.len(), 1);
}

#[test]
fn test_token_budget() {
    let mem = WorkingMemory::new(100);

    mem.add_thought("Short");
    assert!(mem.token_usage() > 0);
    assert!(mem.remaining_budget() < 100);
}

#[test]
fn test_search_memory() {
    let mem = WorkingMemory::new(1000);

    mem.add_thought("Rust programming language");
    mem.add_observation("Python is popular");
    mem.add_result("JavaScript runs in browsers");

    let results = mem.search("rust");
    assert!(!results.is_empty());
    assert_eq!(results[0].0.content, "Rust programming language");
}

#[test]
fn test_tags() {
    let mem = WorkingMemory::new(1000);

    let item = MemoryItem::thought("Tagged thought")
        .with_tags(vec!["important".to_string(), "test".to_string()]);
    mem.add(item);

    let tagged = mem.filter_by_tag("important");
    assert_eq!(tagged.len(), 1);

    let tagged_test = mem.filter_by_tag("test");
    assert_eq!(tagged_test.len(), 1);

    let missing = mem.filter_by_tag("nonexistent");
    assert!(missing.is_empty());
}

#[test]
fn test_importance() {
    let mem = WorkingMemory::new(1000);

    mem.add(MemoryItem::thought("Low importance").with_importance(0.1));
    mem.add(MemoryItem::thought("High importance").with_importance(0.9));

    let results = mem.search("importance");
    assert_eq!(results.len(), 2);
    assert!(results[0].1 > results[1].1);
    assert_eq!(results[0].0.content, "High importance");
}

#[test]
fn test_remove() {
    let mem = WorkingMemory::new(1000);

    let id = mem.add_thought("To be removed");
    assert_eq!(mem.item_count(), 1);

    assert!(mem.remove(&id));
    assert_eq!(mem.item_count(), 0);
    assert!(mem.get(&id).is_none());

    assert!(!mem.remove(&id));
}

#[test]
fn test_clear() {
    let mem = WorkingMemory::new(1000);

    mem.add_thought("One");
    mem.add_thought("Two");
    assert_eq!(mem.item_count(), 2);

    mem.clear();
    assert_eq!(mem.item_count(), 0);
    assert_eq!(mem.token_usage(), 0);
}

#[test]
fn test_render_as_prompt() {
    let mem = WorkingMemory::new(1000);

    mem.add_thought("First thought");
    mem.add_observation("Observation one");

    let rendered = mem.render_as_prompt();
    assert!(rendered.contains("THOUGHT"));
    assert!(rendered.contains("OBSERVATION"));
    assert!(rendered.contains("First thought"));
}

#[test]
fn test_render_with_limit() {
    let mem = WorkingMemory::new(10000);

    for i in 0..10 {
        mem.add_thought(format!("Thought number {}", i));
    }

    let limited = mem.render_with_limit(50);
    assert!(!limited.is_empty());
}

#[test]
fn test_goal_and_plan() {
    let mem = WorkingMemory::new(1000);

    let goal_item = MemoryItem::goal("Build a Rust project");
    let goal_id = goal_item.id.clone();
    mem.add(goal_item);

    let plan_item = MemoryItem::plan("Step by step plan");
    let plan_id = plan_item.id.clone();
    mem.add(plan_item);

    let goal = mem.get(&goal_id).unwrap();
    assert_eq!(goal.importance, 0.9);

    let plan = mem.get(&plan_id).unwrap();
    assert_eq!(plan.importance, 0.7);
}

#[test]
fn test_estimate_tokens() {
    let tokens = item::estimate_tokens("Hello world");
    assert!(tokens > 0);

    let more_tokens = item::estimate_tokens("This is a much longer text with more words and characters");
    assert!(more_tokens > tokens);
}

#[test]
fn test_metadata() {
    let mem = WorkingMemory::new(1000);

    let item = MemoryItem::thought("With metadata")
        .with_metadata(json!({ "source": "test", "count": 42 }));
    mem.add(item);

    let retrieved = mem.filter_by_type(MemoryItemType::Thought);
    assert_eq!(retrieved[0].metadata["count"], 42);
    assert_eq!(retrieved[0].metadata["source"], "test");
}