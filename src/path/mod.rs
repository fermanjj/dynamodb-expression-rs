use core::{
    fmt::{self, Write},
    mem,
    str::FromStr,
};

use itertools::Itertools;

use super::name::Name;

/// Represents a DynamoDB [document path][1]. For example, `foo[3][7].bar[2].baz`.
///
/// Create an instance using the
///
/// When used in an [`Expression`], attribute names in a `Path` are
/// automatically handled as [expression attribute names][2], allowing for names
/// that would not otherwise be permitted by DynamoDB. For example,
/// `foo[3][7].bar[2].baz` would become something similar to `#0[3][7].#1[2].#2`,
/// and the names would be in the `expression_attribute_names`.
///
/// See also: [`Name`]
///
/// # Examples
///
/// Each of these are ways to create a `Path` instance for `foo[3][7].bar[2].baz`.
/// ```
/// use dynamodb_expression::{path::Element, Path};
/// # use pretty_assertions::assert_eq;
/// #
/// # let expected: Path = [
/// #     Element::from(("foo", [3, 7])),
/// #     Element::from(("bar", 2)),
/// #     Element::from("baz"),
/// # ]
/// # .into_iter()
/// # .collect();
///
/// let path: Path = "foo[3][7].bar[2].baz".parse().unwrap();
/// # assert_eq!(expected, path);
///
/// // `Path` implements `FromIterator` for anything that is `Into<Element>`.
/// let path = Path::from_iter([("foo", vec![3, 7]), ("bar", vec![2]), ("baz", vec![])]);
/// # assert_eq!(expected, path);
///
/// // Of course, that means you can `.collect()` into a `Path`.
/// let path: Path = [("foo", vec![3, 7]), ("bar", vec![2]), ("baz", vec![])]
///     .into_iter()
///     .collect();
/// # assert_eq!(expected, path);
///
/// // `Element` can be converted into from strings (`String`, `&str`, `&String`),
/// // as well as string/index tuples. In this case an "index" is an array, slice,
/// // `Vec` of, or a single `u32`.
/// let path: Path = [
///     Element::from(("foo", [3, 7])),
///     Element::from(("bar", 2)),
///     Element::from("baz"),
/// ]
/// .into_iter()
/// .collect();
/// # assert_eq!(expected, path);
///
/// let path: Path = [
///     Element::indexed_field("foo", [3, 7]),
///     Element::indexed_field("bar", 2),
///     Element::name("baz"),
/// ]
/// .into_iter()
/// .collect();
/// # assert_eq!(expected, path);
/// ```
///
/// // TODO: Doc examples for creating instances. From, parse, literals.
/// //       Including for `IndexedField`s.
///
/// [1]: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Expressions.Attributes.html#Expressions.Attributes.NestedElements.DocumentPathExamples
/// [2]: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Expressions.ExpressionAttributeNames.html
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    pub path: Vec<Element>,
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        self.path.iter().try_for_each(|elem| {
            if first {
                first = false;
            } else {
                f.write_char('.')?;
            }

            elem.fmt(f)
        })
    }
}

impl<T> From<T> for Path
where
    T: Into<Element>,
{
    fn from(value: T) -> Self {
        Path {
            path: vec![value.into()],
        }
    }
}

impl<T> FromIterator<T> for Path
where
    T: Into<Element>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self {
            path: iter.into_iter().map(Into::into).collect(),
        }
    }
}

impl FromStr for Path {
    type Err = PathParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            path: s.split('.').map(str::parse).try_collect()?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid document path")]
pub struct PathParseError;

/// Represents one segment in a DynamoDB document [`Path`]. For example, in
/// `foo[3][7].bar[2].baz`, the `Element`s would be `foo[3][7]`, `bar[2]`, and
/// `baz`.
///
/// See [`Path`] for more.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Element {
    Name(Name),
    IndexedField(IndexedField),
}

impl Element {
    pub fn name<N>(name: N) -> Self
    where
        N: Into<Name>,
    {
        Self::Name(name.into())
    }

    pub fn indexed_field<N, I>(name: N, indexes: I) -> Self
    where
        N: Into<Name>,
        I: Indexes,
    {
        let indexes = indexes.into_indexes();
        if indexes.is_empty() {
            Self::name(name)
        } else {
            Self::IndexedField(IndexedField {
                name: name.into(),
                indexes: indexes.into_indexes(),
            })
        }
    }
}

impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Element::Name(name) => name.fmt(f),
            Element::IndexedField(field_index) => field_index.fmt(f),
        }
    }
}

impl FromStr for Element {
    type Err = PathParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut remaining = input;
        let mut name = None;
        let mut indexes = Vec::new();
        while !remaining.is_empty() {
            let open = remaining.find('[');
            let close = remaining.find(']');

            match (open, close) {
                (None, None) => {
                    if name.is_some() {
                        // `bar` in `foo[0]bar`
                        return Err(PathParseError);
                    }

                    // No more braces. Consume the rest of the string.
                    name = Some(mem::take(&mut remaining));
                    break;
                }
                (None, Some(_close)) => return Err(PathParseError),
                (Some(_open), None) => return Err(PathParseError),
                (Some(open), Some(close)) => {
                    if open >= close {
                        // `foo][`
                        return Err(PathParseError);
                    }

                    if name.is_none() {
                        if open > 0 {
                            name = Some(&remaining[..open]);
                        } else {
                            // The string starts with a '['. E.g.:
                            // `[]foo`
                            return Err(PathParseError);
                        }
                    } else if open > 0 {
                        // We've already got the name but we just found another after a closing bracket.
                        // E.g, `bar[0]` in `foo[7]bar[0]`
                        return Err(PathParseError);
                    }

                    let index: u32 = remaining[open + 1..close]
                        .parse()
                        .map_err(|_| PathParseError)?;
                    indexes.push(index);

                    remaining = &remaining[close + 1..];
                }
            }
        }

        Ok(if indexes.is_empty() {
            Self::Name(input.into())
        } else {
            if !remaining.is_empty() {
                // Shouldn't be able to get there.
                // If we do, something above changed and there's a bug.
                return Err(PathParseError);
            }

            let name = name.ok_or(PathParseError)?;

            Self::IndexedField(IndexedField {
                name: name.into(),
                indexes,
            })
        })
    }
}

impl From<IndexedField> for Element {
    fn from(value: IndexedField) -> Self {
        if value.indexes.is_empty() {
            Self::Name(value.name)
        } else {
            Self::IndexedField(value)
        }
    }
}

impl<N, P> From<(N, P)> for Element
where
    N: Into<Name>,
    P: Indexes,
{
    fn from((name, indexes): (N, P)) -> Self {
        let indexes = indexes.into_indexes();
        if indexes.is_empty() {
            Self::Name(name.into())
        } else {
            Self::IndexedField((name, indexes).into())
        }
    }
}

// This would be ideal, but I think trait specialization is needed for this to be workable.
// impl<T> From<T> for Element
// where
//     T: Into<String>,
// {
//     fn from(name: T) -> Self {
//         Self { name: name.into() }
//     }
// }

impl From<Name> for Element {
    fn from(name: Name) -> Self {
        Self::Name(name)
    }
}

impl From<String> for Element {
    fn from(name: String) -> Self {
        Self::Name(name.into())
    }
}

impl From<&String> for Element {
    fn from(name: &String) -> Self {
        Self::Name(name.into())
    }
}

impl From<&str> for Element {
    fn from(name: &str) -> Self {
        Self::Name(name.into())
    }
}

impl From<&&str> for Element {
    fn from(name: &&str) -> Self {
        Self::Name(name.into())
    }
}

/// Represents a segment of a DynamoDB document [`Path`] that is a name with one
/// or more indexes. For example, in `foo[3][7].bar[2].baz`, the segments
/// `foo[3][7]` and `bar[2]` would both be represented as an `IndexedField`.
///
/// See [`Path`] for more.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexedField {
    pub(crate) name: Name,
    indexes: Vec<u32>,
}

impl fmt::Display for IndexedField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)?;
        self.indexes
            .iter()
            .try_for_each(|index| write!(f, "[{}]", index))
    }
}

impl<N, P> From<(N, P)> for IndexedField
where
    N: Into<Name>,
    P: Indexes,
{
    fn from((name, indexes): (N, P)) -> Self {
        Self {
            name: name.into(),
            indexes: indexes.into_indexes(),
        }
    }
}

pub trait Indexes {
    fn into_indexes(self) -> Vec<u32>;
}

impl Indexes for u32 {
    fn into_indexes(self) -> Vec<u32> {
        vec![self]
    }
}

impl Indexes for Vec<u32> {
    fn into_indexes(self) -> Vec<u32> {
        self
    }
}

impl Indexes for &[u32] {
    fn into_indexes(self) -> Vec<u32> {
        self.to_vec()
    }
}

impl<const N: usize> Indexes for [u32; N] {
    fn into_indexes(self) -> Vec<u32> {
        self.to_vec()
    }
}

impl<const N: usize> Indexes for &[u32; N] {
    fn into_indexes(self) -> Vec<u32> {
        self.to_vec()
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::{assert_eq, assert_str_eq};

    use crate::Name;

    use super::{Element, IndexedField, Path, PathParseError};

    #[test]
    fn parse_path() {
        let path: Path = "foo".parse().unwrap();
        assert_eq!(Path::from(Element::from(Name::from("foo"))), path);

        let path: Path = "foo[0]".parse().unwrap();
        assert_eq!(Path::from(Element::indexed_field("foo", [0])), path);

        let path: Path = "foo[0][3]".parse().unwrap();
        assert_eq!(Path::from(Element::indexed_field("foo", [0, 3])), path);

        let path: Path = "foo[42][37][9]".parse().unwrap();
        assert_eq!(Path::from(Element::indexed_field("foo", [42, 37, 9])), path);

        let path: Path = "foo.bar".parse().unwrap();
        assert_eq!(
            Path::from_iter([Element::name("foo"), Element::name("bar")]),
            path
        );

        let path: Path = "foo[42].bar".parse().unwrap();
        assert_eq!(
            Path::from_iter([Element::indexed_field("foo", 42), Element::name("bar")]),
            path
        );

        let path: Path = "foo.bar[37]".parse().unwrap();
        assert_eq!(
            Path::from_iter([Element::name("foo"), Element::indexed_field("bar", 37)]),
            path
        );

        let path: Path = "foo[42].bar[37]".parse().unwrap();
        assert_eq!(
            Path::from_iter([
                Element::indexed_field("foo", 42),
                Element::indexed_field("bar", 37)
            ]),
            path
        );

        let path: Path = "foo[42][7].bar[37]".parse().unwrap();
        assert_eq!(
            Path::from_iter([
                Element::indexed_field("foo", [42, 7]),
                Element::indexed_field("bar", 37)
            ]),
            path
        );

        let path: Path = "foo[42].bar[37][9]".parse().unwrap();
        assert_eq!(
            Path::from_iter([
                Element::indexed_field("foo", 42),
                Element::indexed_field("bar", [37, 9])
            ]),
            path
        );

        let path: Path = "foo[42][7].bar[37][9]".parse().unwrap();
        assert_eq!(
            Path::from_iter([
                Element::indexed_field("foo", [42, 7]),
                Element::indexed_field("bar", [37, 9])
            ]),
            path
        );

        for prefix in ["foo", "foo[0]", "foo.bar", "foo[0]bar", "foo[0]bar[1]"] {
            for bad_index in ["[9", "[]", "][", "[", "]"] {
                let input = format!("{prefix}{bad_index}");

                match input.parse::<Path>() {
                    Ok(path) => {
                        panic!("Should not have parsed invalid input {input:?} into: {path:?}");
                    }
                    Err(PathParseError) => { /* Got the expected error */ }
                }
            }
        }

        // A few other odds and ends
        "foo[0]bar".parse::<Path>().unwrap_err();
        "foo[0]bar[3]".parse::<Path>().unwrap_err();
        "[0]".parse::<Path>().unwrap_err();
    }

    /// Demonstration/proof of how a `Path` can be expressed to prove usability.
    #[test]
    fn express_path() {
        let _: IndexedField = ("foo", 0).into();
        let _: Element = ("foo", 0).into();
        let _: Path = ("foo", 0).into();
    }

    #[test]
    fn display_name() {
        let path = Element::name("foo");
        assert_str_eq!("foo", path.to_string());
    }

    #[test]
    fn display_indexed() {
        let path = Element::indexed_field("foo", 42);
        assert_str_eq!("foo[42]", path.to_string());

        let path = Element::indexed_field("foo", [42]);
        assert_str_eq!("foo[42]", path.to_string());

        let path = Element::indexed_field("foo", &([42, 37, 9])[..]);
        assert_str_eq!("foo[42][37][9]", path.to_string());
    }

    #[test]
    fn display_path() {
        let path: Path = ["foo", "bar"].into_iter().collect();
        assert_str_eq!("foo.bar", path.to_string());

        let path = Path::from_iter([Element::name("foo"), Element::indexed_field("bar", 42)]);
        assert_str_eq!("foo.bar[42]", path.to_string());

        // TODO: I'm not sure this is a legal path based on these examples:
        //       https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/Expressions.Attributes.html#Expressions.Attributes.NestedElements.DocumentPathExamples
        //       Test whether it's valid and remove this comment or handle it appropriately.
        let path = Path::from_iter([Element::indexed_field("foo", 42), Element::name("bar")]);
        assert_str_eq!("foo[42].bar", path.to_string());
    }
}
