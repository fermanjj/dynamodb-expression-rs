/*!
A crate to help build DynamoDB condition, filter, key condition, and update
expressions in a type-safe way.

[`Expression`] is the type to use for a [DynamoDB expression][1].
[`Path`] represents a [DynamoDB item attribute or document path][2], and has
many methods for building various expressions.
See the integration tests for [querying] and [updating] as a starting place.

[1]: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Expressions.html
[2]: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Expressions.Attributes.html
[querying]: https://github.com/dcormier/dynamodb-expression-rs/blob/b18bc1c/tests/aws_sdk_dynamo.rs#L480-L486
[updating]: https://github.com/dcormier/dynamodb-expression-rs/blob/b18bc1c/tests/aws_sdk_dynamo.rs#L52
*/

// TODO: An example here.

extern crate alloc;

// Re-export the crates publicly exposed in our API
pub use ::aws_sdk_dynamodb;
pub use ::num;

pub mod condition;
pub mod expression;
pub mod key;
pub mod operand;
pub mod path;
pub mod update;
pub mod value;

pub use condition::Comparator;
pub use expression::Expression;
pub use path::Path;
pub use value::{
    binary_set, binary_value, bool_value, null_value, num_set, num_value, ref_value, string_set,
    string_value,
};
