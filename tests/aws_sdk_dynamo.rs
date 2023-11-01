#[cfg(test)]
mod dynamodb;

use std::{collections::HashMap, future::Future, pin::Pin};

use aws_sdk_dynamodb::{
    error::SdkError,
    operation::query::QueryError,
    types::{AttributeValue, ReturnValue},
    Client,
};
use dynamodb_expression::{
    expression::Expression,
    key::Key,
    num_value,
    path::{Element, Name, Path},
    string_set, string_value,
    update::{Add, Delete, Remove, Set, SetAction},
    value::Num,
};
use pretty_assertions::{assert_eq, assert_ne};

use crate::dynamodb::{
    debug::DebugList,
    item::{new_item, ATTR_ID, ATTR_LIST, ATTR_MAP, ATTR_NULL, ATTR_NUM, ATTR_STRING},
    setup::{clean_table, delete_table},
    Config, DebugItem,
};

const ITEM_ID: &str = "sanity item";

#[tokio::test]
async fn query() {
    test("query", |config| Box::pin(test_query(config))).await;
}

async fn test_query(config: &Config) {
    let item = fresh_item(config).await;
    let got = Expression::builder()
        .with_key_condition(Key::from(Name::from(ATTR_ID)).equal(string_value(ITEM_ID)))
        .with_projection(
            // Testing with an empty projection expression to see if:
            // 1. DynamoDB allows it or
            // 2. We handle it properly
            Vec::<Name>::default(),
        )
        .build()
        .query(config.client().await)
        .table_name(config.table_name.clone())
        .send()
        .await
        .expect("Failed to query item")
        .items
        .expect("Where is the item?")
        .pop()
        .expect("Got no items");

    assert_eq!(DebugItem(item), DebugItem(got));
}

#[tokio::test]
async fn update() {
    test("update", |config| Box::pin(test_update(config))).await;
}

/// A name for a field that doesn't exist in generated item from [`new_item`] and [`fresh_item`].
const ATTR_NEW_FIELD: &str = "new_field";

async fn test_update(config: &Config) {
    let client = config.client().await;

    test_update_set(config, client).await;
    test_update_remove(config, client).await;
    test_update_add(config, client).await;
    test_update_delete(config, client).await;
}

async fn test_update_set(config: &Config, client: &Client) {
    let item = fresh_item(config).await;
    assert_eq!(None, item.get(ATTR_NEW_FIELD));

    let update = Expression::builder()
        .with_update(Set::from_iter([
            SetAction::from(Path::name(ATTR_STRING).assign("abcdef")),
            Path::name(ATTR_NUM).math().sub(3.5).into(),
            Path::name(ATTR_LIST)
                .list_append()
                .before()
                .list(["A new value at the beginning"])
                .into(),
            // DynamoDB won't let you append to the same list twice in the same update expression.
            // Path::name(ATTR_LIST)
            //     .list_append()
            //     .list(["A new value at the end"])
            //     .into(),
            Path::name(ATTR_NEW_FIELD)
                .if_not_exists()
                .value("A new field")
                .into(),
        ]))
        .build()
        .update_item(client)
        .table_name(&config.table_name)
        .set_key(item_key(&item).into());

    // println!("{:?}", update.as_input());

    update.send().await.expect("Failed to update item");

    // Once more to add another item to the end of that list.
    // DynamoDB won't allow both in a single update expression.
    let updated_item = Expression::builder()
        .with_update(
            Path::name(ATTR_LIST)
                .list_append()
                .list(["A new value at the end"]),
        )
        .build()
        .update_item(client)
        .set_key(item_key(&item).into())
        .table_name(&config.table_name)
        .return_values(ReturnValue::AllNew)
        .send()
        .await
        .expect("Failed to update item")
        .attributes
        .expect("Where is the item?");

    // println!("Got item: {:#?}", DebugItem(&after_update));

    assert_ne!(
        item.get(ATTR_STRING),
        updated_item.get(ATTR_STRING),
        "Updated string should be different."
    );
    assert_eq!(
        "abcdef",
        updated_item
            .get(ATTR_STRING)
            .map(AttributeValue::as_s)
            .expect("Field is missing")
            .expect("That field should be a String"),
        "Assigning a new value to the field didn't work"
    );
    assert_ne!(
        item.get(ATTR_NUM),
        updated_item.get(ATTR_NUM),
        "Updated number should be different"
    );
    assert_eq!(
        "38.5",
        updated_item
            .get(ATTR_NUM)
            .map(AttributeValue::as_n)
            .expect("Field is missing")
            .expect("That field should be a Number"),
        "Subtraction didn't work"
    );
    assert_eq!(
        "A new field",
        updated_item
            .get(ATTR_NEW_FIELD)
            .map(AttributeValue::as_s)
            .expect("Field is missing")
            .expect("The field should be a string"),
        "The new field was not added to the item as expected"
    );

    let updated_list = updated_item
        .get(ATTR_LIST)
        .map(AttributeValue::as_l)
        .expect("List is missing")
        .expect("The field should be a list");
    assert_eq!(
        item.get(ATTR_LIST)
            .map(AttributeValue::as_l)
            .expect("List is missing")
            .expect("The field should be a list")
            .len()
            + 2,
        updated_list.len(),
        "The list should have had two items added to it"
    );
    assert_eq!(
        Some(&AttributeValue::S("A new value at the beginning".into())),
        updated_list.first(),
        "List is missing the new item at the beginning: {:#?}",
        DebugList(updated_list)
    );
    assert_eq!(
        Some(&AttributeValue::S("A new value at the end".into())),
        updated_list.last(),
        "List is missing the new item at th end: {:#?}",
        DebugList(updated_list)
    );
}

async fn test_update_remove(config: &Config, client: &Client) {
    let item = fresh_item(config).await;
    assert_eq!(None, item.get(ATTR_NEW_FIELD));

    let update = Expression::builder()
        .with_update(Remove::from_iter([
            Element::name(ATTR_NULL).into(),
            Path::from_iter([
                Element::name(ATTR_MAP),
                Element::indexed_field(ATTR_LIST, 0),
            ]),
            Path::from_iter([ATTR_MAP, ATTR_NULL].map(Name::from)),
        ]))
        .build()
        .update_item(client)
        .table_name(&config.table_name)
        .set_key(item_key(&item).into());

    // println!("\n{:?}\n", update.as_input());

    let updated_item = update
        .return_values(ReturnValue::AllNew)
        .send()
        .await
        .expect("Failed to update item")
        .attributes
        .expect("Where is the item?");

    // println!("Got item: {:#?}", DebugItem(&updated_item));

    assert_eq!(
        None,
        updated_item.get(ATTR_NULL),
        "Attribute should have been removed"
    );

    let map_updated = updated_item
        .get(ATTR_MAP)
        .expect("Map attribute is missing")
        .as_m()
        .expect("Field is not a map");

    assert_eq!(
        None,
        map_updated.get(ATTR_NULL),
        "Sub-attribute should have been removed"
    );

    let map_list = item
        .get(ATTR_MAP)
        .expect("Map attribute is missing")
        .as_m()
        .expect("Field is not a map")
        .get(ATTR_LIST)
        .expect("List is missing from the map")
        .as_l()
        .expect("Item is not a list");

    let map_list_updated = map_updated
        .get(ATTR_LIST)
        .expect("List is missing from the map")
        .as_l()
        .expect("Item is not a list");

    assert_eq!(
        map_list.len() - 1,
        map_list_updated.len(),
        "There should have been one item removed"
    );

    let map_list_first = map_list.first().unwrap();
    // println!(
    //     "Looking to see if this was removed: {:?}",
    //     DebugAttributeValue(map_list_first)
    // );
    assert_eq!(
        None,
        map_list_updated.iter().find(|elem| *elem == map_list_first),
        "The first item should have been removed"
    );
}

async fn test_update_add(config: &Config, client: &Client) {
    let item = fresh_item(config).await;
    assert_eq!(None, item.get(ATTR_NEW_FIELD));

    let update = Expression::builder()
        .with_update(Add::new(
            Path::from_iter([
                Element::name(ATTR_MAP),
                Element::indexed_field(ATTR_LIST, 1),
            ]),
            string_set(["d", "e", "f"]),
        ))
        .build()
        .update_item(client)
        .table_name(&config.table_name)
        .set_key(item_key(&item).into());

    // println!("\n{:?}\n", update.as_input());

    update.send().await.expect("Failed to update item");

    let update = Expression::builder()
        .with_update(Add::new(
            Path::from_iter([
                Element::name(ATTR_MAP),
                Element::indexed_field(ATTR_LIST, 2),
            ]),
            // TODO: Can it be made so `num_value()` can be used here?
            Num::new(-3.5),
        ))
        .build()
        .update_item(client)
        .table_name(&config.table_name)
        .set_key(item_key(&item).into());

    // println!("\n{:?}\n", update.as_input());

    let updated_item = update
        .return_values(ReturnValue::AllNew)
        .send()
        .await
        .expect("Failed to update item")
        .attributes
        .expect("Where is the item?");

    // println!("Got item: {:#?}", DebugItem(&updated_item));

    let map_list = item
        .get(ATTR_MAP)
        .expect("Map attribute is missing")
        .as_m()
        .expect("Field is not a map")
        .get(ATTR_LIST)
        .expect("List is missing from the map")
        .as_l()
        .expect("Item is not a list");

    let map_list_updated = updated_item
        .get(ATTR_MAP)
        .expect("Map attribute is missing")
        .as_m()
        .expect("Field is not a map")
        .get(ATTR_LIST)
        .expect("List is missing from the map")
        .as_l()
        .expect("Item is not a list");

    let ss = map_list
        .get(1)
        .expect("Item doesn't exist")
        .as_ss()
        .expect("Item is not a string set");

    let ss_updated = map_list_updated
        .get(1)
        .expect("Item doesn't exist")
        .as_ss()
        .expect("Item is not a string set");

    assert_eq!(3, ss.len());
    assert_eq!(6, ss_updated.len());
    assert_eq!(["a", "b", "c", "d", "e", "f"], ss_updated.as_slice());

    let n = map_list
        .get(2)
        .expect("Item doesn't exist")
        .as_n()
        .expect("Item is not a number");

    let n_updated = map_list_updated
        .get(2)
        .expect("Item doesn't exist")
        .as_n()
        .expect("Item is not a number");

    assert_eq!("42", n);
    assert_eq!("38.5", n_updated);
}

async fn test_update_delete(config: &Config, client: &Client) {
    let item = fresh_item(config).await;
    assert_eq!(None, item.get(ATTR_NEW_FIELD));

    let update = Expression::builder()
        .with_update(Delete::new(
            Path::from_iter([
                Element::name(ATTR_MAP),
                Element::indexed_field(ATTR_LIST, 1),
            ]),
            string_set(["a", "c", "d"]),
        ))
        .build()
        .update_item(client)
        .table_name(&config.table_name)
        .set_key(item_key(&item).into());

    // println!("\n{:?}\n", update.as_input());

    let updated_item = update
        .return_values(ReturnValue::AllNew)
        .send()
        .await
        .expect("Failed to update item")
        .attributes
        .expect("Where is the item?");

    // println!("Got item: {:#?}", DebugItem(&updated_item));

    let map_list = item
        .get(ATTR_MAP)
        .expect("Map attribute is missing")
        .as_m()
        .expect("Field is not a map")
        .get(ATTR_LIST)
        .expect("List is missing from the map")
        .as_l()
        .expect("Item is not a list");

    let map_list_updated = updated_item
        .get(ATTR_MAP)
        .expect("Map attribute is missing")
        .as_m()
        .expect("Field is not a map")
        .get(ATTR_LIST)
        .expect("List is missing from the map")
        .as_l()
        .expect("Item is not a list");

    let ss = map_list
        .get(1)
        .expect("Item doesn't exist")
        .as_ss()
        .expect("Item is not a string set");

    let ss_updated = map_list_updated
        .get(1)
        .expect("Item doesn't exist")
        .as_ss()
        .expect("Item is not a string set");

    assert_eq!(3, ss.len());
    assert_eq!(1, ss_updated.len());
    assert_eq!(["b"], ss_updated.as_slice());
}

/// Wraps a test function in code to set up and tear down the DynamoDB table.
///
/// The `name` value must be safe for use as a DynamoDB table name.
async fn test<F, T>(name: &str, test_fn: F) -> T
where
    F: FnOnce(&Config) -> Pin<Box<dyn Future<Output = T> + '_>>,
{
    let mut config = Config::new_local();
    config.table_name = format!("{}-{}", config.table_name, name);
    let config = config; // No longer mutable.
    let client = config.client().await;

    clean_table(client, &config.table_name)
        .await
        .expect("error creating table");

    let result = (test_fn)(&config).await;

    delete_table(client, &config.table_name)
        .await
        .expect("error deleting table");

    result
}

/// Deletes the item (if it exists) from [`new_item`] and inserts a new one.
/// Returns the inserted item.
async fn fresh_item(config: &Config) -> HashMap<String, AttributeValue> {
    let item = new_item(ITEM_ID);

    config
        .client()
        .await
        .put_item()
        .table_name(&config.table_name)
        .set_item(Some(item.clone()))
        .send()
        .await
        .expect("Failed to put item");

    item
}

/// Gets the item key for the given item.
fn item_key(item: &HashMap<String, AttributeValue>) -> HashMap<String, AttributeValue> {
    [(ATTR_ID.into(), item[ATTR_ID].clone())].into()
}

/// Gets the item from the configured table
async fn get_item(
    config: &Config,
) -> Result<Option<HashMap<String, AttributeValue>>, SdkError<QueryError>> {
    Expression::builder()
        .with_key_condition(Key::from(Name::from(ATTR_ID)).equal(string_value(ITEM_ID)))
        .build()
        .query(config.client().await)
        .table_name(config.table_name.clone())
        .send()
        .await
        .map(|resp| {
            let mut items = resp.items.expect("Should have found items");

            assert!(
                items.len() <= 1,
                "Should not have gotten more than one item"
            );

            items.pop()
        })
}
