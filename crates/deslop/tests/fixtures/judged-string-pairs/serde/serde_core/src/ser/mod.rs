/// [example data format]: https://serde.rs/data-format.html
#[cfg_attr(
    not(no_diagnostic_namespace),
    diagnostic::on_unimplemented(
        message = "the trait bound `{Self}: serde::ser::SerializeSeq` is not satisfied",
    )
)]
pub trait SerializeSeq {
    /// Must match the `Ok` type of our `Serializer`.
    type Ok;

    /// Must match the `Error` type of our `Serializer`.
    type Error: Error;

    /// Serialize a sequence element.
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize;

    /// Finish serializing a sequence.
    fn end(self) -> Result<Self::Ok, Self::Error>;
}

/// Returned from `Serializer::serialize_tuple`.
///
/// # Example use
///
/// ```edition2021
/// use serde::ser::{Serialize, SerializeTuple, Serializer};
///
/// # mod fool {
/// #     trait Serialize {}
/// impl<A, B, C> Serialize for (A, B, C)
/// #     {}
/// # }
/// #
/// # struct Tuple3<A, B, C>(A, B, C);
/// #
/// # impl<A, B, C> Serialize for Tuple3<A, B, C>
/// where
///     A: Serialize,
///     B: Serialize,
///     C: Serialize,
/// {
///     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
///     where
///         S: Serializer,
///     {
///         let mut tup = serializer.serialize_tuple(3)?;
///         tup.serialize_element(&self.0)?;
///         tup.serialize_element(&self.1)?;
///         tup.serialize_element(&self.2)?;
///         tup.end()
///     }
/// }
/// ```
///
/// ```edition2021
/// # use std::marker::PhantomData;
/// #
/// # struct Array<T>(PhantomData<T>);
/// #
/// # impl<T> Array<T> {
/// #     fn len(&self) -> usize {
/// #         unimplemented!()
/// #     }
/// # }
/// #
/// # impl<'a, T> IntoIterator for &'a Array<T> {
/// #     type Item = &'a T;
/// #     type IntoIter = Box<dyn Iterator<Item = &'a T>>;
/// #     fn into_iter(self) -> Self::IntoIter {
/// #         unimplemented!()
/// #     }
/// # }
/// #
/// use serde::ser::{Serialize, SerializeTuple, Serializer};
///
/// # mod fool {
/// #     trait Serialize {}
/// impl<T> Serialize for [T; 16]
/// #     {}
/// # }
/// #
/// # impl<T> Serialize for Array<T>
/// where
///     T: Serialize,
/// {
///     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
///     where
///         S: Serializer,
///     {
///         let mut seq = serializer.serialize_tuple(16)?;
///         for element in self {
///             seq.serialize_element(element)?;
///         }
///         seq.end()
///     }
/// }
/// ```
///
/// # Example implementation
///
/// The [example data format] presented on the website demonstrates an
/// implementation of `SerializeTuple` for a basic JSON data format.
///
/// [example data format]: https://serde.rs/data-format.html
#[cfg_attr(
    not(no_diagnostic_namespace),
    diagnostic::on_unimplemented(
        message = "the trait bound `{Self}: serde::ser::SerializeTuple` is not satisfied",
    )
)]
pub trait SerializeTuple {
    /// Must match the `Ok` type of our `Serializer`.
    type Ok;

    /// Must match the `Error` type of our `Serializer`.
    type Error: Error;

    /// Serialize a tuple element.
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize;

    /// Finish serializing a tuple.
    fn end(self) -> Result<Self::Ok, Self::Error>;
}

/// Returned from `Serializer::serialize_tuple_struct`.
///
/// # Example use
///
/// ```edition2021
/// use serde::ser::{Serialize, SerializeTupleStruct, Serializer};
///
/// struct Rgb(u8, u8, u8);
///
/// impl Serialize for Rgb {
///     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
///     where
///         S: Serializer,
///     {
///         let mut ts = serializer.serialize_tuple_struct("Rgb", 3)?;
///         ts.serialize_field(&self.0)?;
///         ts.serialize_field(&self.1)?;
///         ts.serialize_field(&self.2)?;
///         ts.end()
///     }
/// }
/// ```
///
/// # Example implementation
///
/// The [example data format] presented on the website demonstrates an
/// implementation of `SerializeTupleStruct` for a basic JSON data format.
///
/// [example data format]: https://serde.rs/data-format.html
#[cfg_attr(
    not(no_diagnostic_namespace),
    diagnostic::on_unimplemented(
        message = "the trait bound `{Self}: serde::ser::SerializeTupleStruct` is not satisfied",
    )
)]
pub trait SerializeTupleStruct {
    /// Must match the `Ok` type of our `Serializer`.
    type Ok;

    /// Must match the `Error` type of our `Serializer`.
    type Error: Error;

    /// Serialize a tuple struct field.
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize;

    /// Finish serializing a tuple struct.
    fn end(self) -> Result<Self::Ok, Self::Error>;
}

/// Returned from `Serializer::serialize_tuple_variant`.
///
/// # Example use
///
/// ```edition2021
/// use serde::ser::{Serialize, SerializeTupleVariant, Serializer};
///
/// enum E {
///     T(u8, u8),
///     U(String, u32, u32),
/// }
///
/// impl Serialize for E {
///     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
///     where
///         S: Serializer,
///     {
///         match *self {
///             E::T(ref a, ref b) => {
///                 let mut tv = serializer.serialize_tuple_variant("E", 0, "T", 2)?;
///                 tv.serialize_field(a)?;
///                 tv.serialize_field(b)?;
///                 tv.end()
///             }
///             E::U(ref a, ref b, ref c) => {
///                 let mut tv = serializer.serialize_tuple_variant("E", 1, "U", 3)?;
///                 tv.serialize_field(a)?;
///                 tv.serialize_field(b)?;
///                 tv.serialize_field(c)?;
///                 tv.end()
///             }
///         }
///     }
/// }
/// ```
///
/// # Example implementation
///
/// The [example data format] presented on the website demonstrates an
/// implementation of `SerializeTupleVariant` for a basic JSON data format.
///
/// [example data format]: https://serde.rs/data-format.html
#[cfg_attr(
    not(no_diagnostic_namespace),
    diagnostic::on_unimplemented(
        message = "the trait bound `{Self}: serde::ser::SerializeTupleVariant` is not satisfied",
    )
)]
pub trait SerializeTupleVariant {
    /// Must match the `Ok` type of our `Serializer`.
    type Ok;

    /// Must match the `Error` type of our `Serializer`.
    type Error: Error;

    /// Serialize a tuple variant field.
    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize;

    /// Finish serializing a tuple variant.
    fn end(self) -> Result<Self::Ok, Self::Error>;
}
