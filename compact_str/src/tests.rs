use std::borrow::Cow;
use std::num;
use std::str::FromStr;

use proptest::prelude::*;
use proptest::strategy::Strategy;
use test_strategy::proptest;

use crate::{
    CompactString,
    ToCompactString,
};

#[cfg(target_pointer_width = "64")]
const MAX_SIZE: usize = 24;
#[cfg(target_pointer_width = "32")]
const MAX_SIZE: usize = 12;

/// generates random unicode strings, upto 80 chars long
pub fn rand_unicode() -> impl Strategy<Value = String> {
    proptest::collection::vec(proptest::char::any(), 0..80).prop_map(|v| v.into_iter().collect())
}

/// generates a random collection of bytes, upto 80 bytes long
pub fn rand_bytes() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(any::<u8>(), 0..80)
}

/// [`proptest::strategy::Strategy`] that generates [`String`]s with up to `len` bytes
pub fn rand_unicode_with_max_len(len: usize) -> impl Strategy<Value = String> {
    proptest::collection::vec(proptest::char::any(), 0..len).prop_map(move |chars| {
        let mut len_utf8 = 0;
        chars
            .into_iter()
            .take_while(|c| {
                len_utf8 += c.len_utf8();
                len_utf8 <= len
            })
            .collect::<String>()
    })
}

/// generates groups upto 40 strings long of random unicode strings, upto 80 chars long
fn rand_unicode_collection() -> impl Strategy<Value = Vec<String>> {
    proptest::collection::vec(rand_unicode(), 0..40)
}

/// Asserts a [`CompactString`] is allocated properly
fn assert_allocated_properly(compact: &CompactString) {
    if compact.len() <= MAX_SIZE {
        assert!(!compact.is_heap_allocated())
    } else {
        assert!(compact.is_heap_allocated())
    }
}

#[proptest]
#[cfg_attr(miri, ignore)]
fn proptest_strings_roundtrip(#[strategy(rand_unicode())] word: String) {
    let compact = CompactString::new(&word);
    prop_assert_eq!(&word, &compact);
}

#[proptest]
#[cfg_attr(miri, ignore)]
fn proptest_strings_allocated_properly(#[strategy(rand_unicode())] word: String) {
    let compact = CompactString::new(&word);
    assert_allocated_properly(&compact);
}

#[proptest]
#[cfg_attr(miri, ignore)]
fn proptest_char_iterator_roundtrips(#[strategy(rand_unicode())] word: String) {
    let compact: CompactString = word.clone().chars().collect();
    prop_assert_eq!(&word, &compact)
}

#[proptest]
#[cfg_attr(miri, ignore)]
fn proptest_string_iterator_roundtrips(
    #[strategy(rand_unicode_collection())] collection: Vec<String>,
) {
    let compact: CompactString = collection.clone().into_iter().collect();
    let word: String = collection.into_iter().collect();
    prop_assert_eq!(&word, &compact);
}

#[proptest]
#[cfg_attr(miri, ignore)]
fn proptest_from_bytes_roundtrips(#[strategy(rand_unicode())] word: String) {
    let bytes = word.into_bytes();
    let compact = CompactString::from_utf8(&bytes).unwrap();
    let word = String::from_utf8(bytes).unwrap();

    prop_assert_eq!(compact, word);
}

#[proptest]
#[cfg_attr(miri, ignore)]
fn proptest_from_bytes_only_valid_utf8(#[strategy(rand_bytes())] bytes: Vec<u8>) {
    let compact_result = CompactString::from_utf8(&bytes);
    let word_result = String::from_utf8(bytes);

    match (compact_result, word_result) {
        (Ok(c), Ok(s)) => prop_assert_eq!(c, s),
        (Err(c_err), Err(s_err)) => prop_assert_eq!(c_err, s_err.utf8_error()),
        _ => panic!("CompactString and core::str read UTF-8 differently?"),
    }
}

#[proptest]
#[cfg_attr(miri, ignore)]
fn proptest_from_lossy_cow_roundtrips(#[strategy(rand_bytes())] bytes: Vec<u8>) {
    let cow = String::from_utf8_lossy(&bytes[..]);
    let compact = CompactString::from(cow.clone());
    prop_assert_eq!(cow, compact);
}

#[proptest]
#[cfg_attr(miri, ignore)]
fn proptest_reserve_and_write_bytes(#[strategy(rand_unicode())] word: String) {
    let mut compact = CompactString::default();
    prop_assert!(compact.is_empty());

    // reserve enough space to write our bytes
    compact.reserve(word.len());

    // SAFETY: We're writing a String which we know is UTF-8
    let slice = unsafe { compact.as_mut_bytes() };
    slice[..word.len()].copy_from_slice(word.as_bytes());

    // SAFTEY: We know this is the length of our string, since `compact` started with 0 bytes
    // and we just wrote `word.len()` bytes
    unsafe { compact.set_len(word.len()) }

    prop_assert_eq!(&word, &compact);
}

#[proptest]
#[cfg_attr(miri, ignore)]
fn proptest_reserve_and_write_bytes_allocated_properly(#[strategy(rand_unicode())] word: String) {
    let mut compact = CompactString::default();
    prop_assert!(compact.is_empty());

    // reserve enough space to write our bytes
    compact.reserve(word.len());

    // SAFETY: We're writing a String which we know is UTF-8
    let slice = unsafe { compact.as_mut_bytes() };
    slice[..word.len()].copy_from_slice(word.as_bytes());

    // SAFTEY: We know this is the length of our string, since `compact` started with 0 bytes
    // and we just wrote `word.len()` bytes
    unsafe { compact.set_len(word.len()) }

    prop_assert_eq!(compact.len(), word.len());

    // The string should be heap allocated if `word` was > MAX_SIZE
    //
    // NOTE: The reserve and write API's don't currently support the Packed representation
    prop_assert_eq!(compact.is_heap_allocated(), word.len() > MAX_SIZE);
}

#[proptest]
#[cfg_attr(miri, ignore)]
fn proptest_arbitrary_compact_string_converts_to_string(#[strategy(rand_unicode())] word: String) {
    let compact = CompactString::new(&word);
    let result = String::from(compact);

    prop_assert_eq!(result.len(), word.len());
    prop_assert_eq!(result, word);
}

#[proptest]
#[cfg_attr(miri, ignore)]
fn proptest_extend_chars_allocated_properly(
    #[strategy(rand_unicode())] start: String,
    #[strategy(rand_unicode())] extend: String,
) {
    let mut compact = CompactString::new(&start);
    compact.extend(extend.chars());

    let mut control = start.clone();
    control.extend(extend.chars());

    prop_assert_eq!(&compact, &control);
    assert_allocated_properly(&compact);
}

#[proptest]
#[cfg_attr(miri, ignore)]
fn proptest_truncate(#[strategy(rand_unicode())] mut control: String, val: u8) {
    let initial_len = control.len();
    let mut compact = CompactString::new(&control);

    // turn the arbitrary number `val` into character indices
    let new_len = control
        .char_indices()
        .into_iter()
        .cycle()
        .nth(val as usize)
        .unwrap_or_default()
        .0;

    // then truncate both strings string
    control.truncate(new_len);
    compact.truncate(new_len);

    // assert they're equal
    prop_assert_eq!(&control, &compact);
    prop_assert_eq!(control.len(), compact.len());

    // If we started as heap allocated, we should stay heap allocated. This prevents us from
    // needing to deallocate the buffer on the heap
    if initial_len > MAX_SIZE {
        prop_assert!(compact.is_heap_allocated());
    } else {
        prop_assert!(!compact.is_heap_allocated());
    }
}

#[test]
fn test_const_creation() {
    const EMPTY: CompactString = CompactString::new_inline("");
    const SHORT: CompactString = CompactString::new_inline("rust");

    #[cfg(target_pointer_width = "64")]
    const PACKED: CompactString = CompactString::new_inline("i am 24 characters long!");
    #[cfg(target_pointer_width = "32")]
    const PACKED: CompactString = CompactString::new_inline("i am 12 char");

    assert_eq!(EMPTY, CompactString::new(""));
    assert_eq!(SHORT, CompactString::new("rust"));

    #[cfg(target_pointer_width = "64")]
    assert_eq!(PACKED, CompactString::new("i am 24 characters long!"));
    #[cfg(target_pointer_width = "32")]
    assert_eq!(PACKED, CompactString::new("i am 12 char"));
}

#[test]
fn test_short_ascii() {
    // always inlined on all archs
    let strs = vec!["nyc", "statue", "liberty", "img_1234.png"];

    for s in strs {
        let compact = CompactString::new(s);
        assert_eq!(compact, s);
        assert_eq!(s, compact);
        assert_eq!(compact.is_heap_allocated(), false);
    }
}

#[test]
fn test_short_unicode() {
    let strs = vec![
        ("🦀", false),
        ("🌧☀️", false),
        // str is 12 bytes long, and leading character is non-ASCII
        ("咬𓅈ꁈ:_", false),
    ];

    for (s, is_heap) in strs {
        let compact = CompactString::new(s);
        assert_eq!(compact, s);
        assert_eq!(s, compact);
        assert_eq!(compact.is_heap_allocated(), is_heap);
    }
}

#[test]
fn test_medium_ascii() {
    let strs = vec![
        "rustconf 2021",
        "new york city",
        "nyc pizza is good",
        "test the 24 char limit!!",
    ];

    for s in strs {
        let compact = CompactString::new(s);
        assert_eq!(compact, s);
        assert_eq!(s, compact);

        #[cfg(target_pointer_width = "64")]
        let is_heap = false;
        #[cfg(target_pointer_width = "32")]
        let is_heap = true;
        assert_eq!(compact.is_heap_allocated(), is_heap);
    }
}

#[test]
fn test_medium_unicode() {
    let strs = vec![
        ("☕️👀😁🎉", false),
        // str is 24 bytes long, and leading character is non-ASCII
        ("🦀😀😃😄😁🦀", false),
    ];

    #[allow(unused_variables)]
    for (s, is_heap) in strs {
        let compact = CompactString::new(s);
        assert_eq!(compact, s);
        assert_eq!(s, compact);

        #[cfg(target_pointer_width = "64")]
        let is_heap = is_heap;
        #[cfg(target_pointer_width = "32")]
        let is_heap = true;

        assert_eq!(compact.is_heap_allocated(), is_heap);
    }
}

#[test]
fn test_from_str_trait() {
    let s = "hello_world";

    // Until the never type `!` is stabilized, we have to unwrap here
    let c = CompactString::from_str(s).unwrap();

    assert_eq!(s, c);
}

#[test]
#[cfg_attr(target_pointer_width = "32", ignore)]
fn test_from_char_iter() {
    let s = "\u{0} 0 \u{0}a𐀀𐀀 𐀀a𐀀";
    println!("{}", s.len());
    let compact: CompactString = s.chars().into_iter().collect();

    assert!(!compact.is_heap_allocated());
    assert_eq!(s, compact);
}

#[test]
#[cfg_attr(target_pointer_width = "32", ignore)]
fn test_extend_packed_from_empty() {
    let s = "  0\u{80}A\u{0}𐀀 𐀀¡a𐀀0";

    let mut compact = CompactString::new(s);
    assert!(!compact.is_heap_allocated());

    // extend from an empty iterator
    compact.extend("".chars());

    // we should still be heap allocated
    assert!(!compact.is_heap_allocated());
}

#[test]
fn test_pop_empty() {
    let num_pops = 256;
    let mut compact = CompactString::from("");

    (0..num_pops).for_each(|_| {
        let ch = compact.pop();
        assert!(ch.is_none());
    });
    assert!(compact.is_empty());
    assert_eq!(compact, "");
}

#[test]
fn test_extend_from_empty_strs() {
    let strs = vec![
        "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
        "", "",
    ];
    let compact: CompactString = strs.clone().into_iter().collect();

    assert_eq!(compact, "");
    assert!(compact.is_empty());
    assert!(!compact.is_heap_allocated());
}

#[test]
fn test_compact_str_is_send_and_sync() {
    fn is_send_and_sync<T: Send + Sync>() {}
    is_send_and_sync::<CompactString>();
}

#[test]
fn test_fmt_write() {
    use core::fmt::Write;

    let mut compact = CompactString::default();

    write!(compact, "test").unwrap();
    assert_eq!(compact, "test");

    writeln!(compact, "{}", 1234).unwrap();
    assert_eq!(compact, "test1234\n");

    write!(compact, "{:>8} {} {:<8}", "some", "more", "words").unwrap();
    assert_eq!(compact, "test1234\n    some more words   ");
}

#[test]
fn test_plus_operator() {
    // + &CompactString
    assert_eq!(CompactString::from("a") + &CompactString::from("b"), "ab");
    // + &str
    assert_eq!(CompactString::from("a") + "b", "ab");
    // + &String
    assert_eq!(CompactString::from("a") + &String::from("b"), "ab");
    // + &Box<str>
    let box_str = String::from("b").into_boxed_str();
    assert_eq!(CompactString::from("a") + &box_str, "ab");
    // + &Cow<'a, str>
    let cow = Cow::from("b");
    assert_eq!(CompactString::from("a") + &cow, "ab");

    // Implementing `Add<T> for String` can break adding &String or other types to String, so we
    // explicitly don't do this. See https://github.com/rust-lang/rust/issues/77143 for more details.
    // Below we assert adding types to String still compiles

    // String + &CompactString
    assert_eq!(String::from("a") + &CompactString::from("b"), "ab");
    // String + &String
    assert_eq!(String::from("a") + &("b".to_string()), "ab");
    // String + &str
    assert_eq!(String::from("a") + &"b", "ab");
}

#[test]
fn test_plus_equals_operator() {
    let mut m = CompactString::from("a");
    m += "b";
    assert_eq!(m, "ab");
}

#[test]
fn test_u8_to_compact_string() {
    let vals = [u8::MIN, 1, 42, u8::MAX - 2, u8::MAX - 1, u8::MAX];

    for x in &vals {
        let c = x.to_compact_string();
        let s = x.to_string();

        assert_eq!(c, s);
        assert!(!c.is_heap_allocated());
    }
}

#[test]
fn test_i8_to_compact_string() {
    let vals = [
        i8::MIN,
        i8::MIN + 1,
        i8::MIN + 2,
        -1,
        0,
        1,
        42,
        i8::MAX - 2,
        i8::MAX - 1,
        i8::MAX,
    ];

    for x in &vals {
        let c = x.to_compact_string();
        let s = x.to_string();

        assert_eq!(c, s);
        assert!(!c.is_heap_allocated());
    }
}

#[test]
fn test_u16_to_compact_string() {
    let vals = [u16::MIN, 1, 42, 999, u16::MAX - 2, u16::MAX - 1, u16::MAX];

    for x in &vals {
        let c = x.to_compact_string();
        let s = x.to_string();

        assert_eq!(c, s);
        assert!(!c.is_heap_allocated());
    }
}

#[test]
fn test_i16_to_compact_string() {
    let vals = [
        i16::MIN,
        i16::MIN + 1,
        i16::MIN + 2,
        -42,
        -1,
        0,
        1,
        42,
        999,
        i16::MAX - 2,
        i16::MAX - 1,
        i16::MAX,
    ];

    for x in &vals {
        let c = x.to_compact_string();
        let s = x.to_string();

        assert_eq!(c, s);
        assert!(!c.is_heap_allocated());
    }
}

#[test]
fn test_u32_to_compact_string() {
    let vals = [
        u32::MIN,
        1,
        42,
        999,
        123456789,
        u32::MAX - 2,
        u32::MAX - 1,
        u32::MAX,
    ];

    for x in &vals {
        let c = x.to_compact_string();
        let s = x.to_string();

        assert_eq!(c, s);
        assert!(!c.is_heap_allocated());
    }
}

#[test]
fn test_i32_to_compact_string() {
    let vals = [
        i32::MIN,
        i32::MIN + 2,
        i32::MIN + 1,
        -12345678,
        -42,
        -1,
        0,
        1,
        999,
        123456789,
        i32::MAX - 2,
        i32::MAX - 1,
        i32::MAX,
    ];

    for x in &vals {
        let c = x.to_compact_string();
        let s = x.to_string();

        assert_eq!(c, s);
        assert!(!c.is_heap_allocated());
    }
}

#[test]
fn test_u64_to_compact_string() {
    let vals = [
        u64::MIN,
        1,
        999,
        123456789,
        98765432123456,
        u64::MAX - 2,
        u64::MAX - 1,
        u64::MAX,
    ];

    for x in &vals {
        let c = x.to_compact_string();
        let s = x.to_string();

        assert_eq!(c, s);

        // u64 can be up-to 20 characters long, which can't be inlined on 32-bit arches
        #[cfg(target_pointer_width = "64")]
        assert!(!c.is_heap_allocated());
    }
}

#[test]
fn test_i64_to_compact_string() {
    let vals = [
        i64::MIN,
        i64::MIN + 1,
        i64::MIN + 2,
        -22222222,
        -42,
        0,
        1,
        999,
        123456789,
        i64::MAX - 2,
        i64::MAX - 1,
        i64::MAX,
    ];

    for x in &vals {
        let c = x.to_compact_string();
        let s = x.to_string();

        assert_eq!(c, s);

        // i64 can be up-to 20 characters long, which can't be inlined on 32-bit arches
        #[cfg(target_pointer_width = "64")]
        assert!(!c.is_heap_allocated());
    }
}

#[test]
fn test_u128_to_compact_string() {
    let vals = [
        u128::MIN,
        1,
        999,
        123456789,
        u128::MAX - 2,
        u128::MAX - 1,
        u128::MAX,
    ];

    for x in &vals {
        let c = x.to_compact_string();
        let s = x.to_string();

        assert_eq!(c, s);
    }
}

#[test]
fn test_i128_to_compact_string() {
    let vals = [
        i128::MIN,
        i128::MIN + 1,
        i128::MIN + 2,
        -22222222,
        -42,
        0,
        1,
        999,
        123456789,
        i128::MAX - 2,
        i128::MAX - 1,
        i128::MAX,
    ];

    for x in &vals {
        let c = x.to_compact_string();
        let s = x.to_string();

        assert_eq!(c, s);
    }
}

#[test]
fn test_bool_to_compact_string() {
    let c = true.to_compact_string();
    let s = true.to_string();

    assert_eq!("true", c);
    assert_eq!(c, s);
    assert!(!c.is_heap_allocated());

    let c = false.to_compact_string();
    let s = false.to_string();

    assert_eq!("false", c);
    assert_eq!(c, s);
    assert!(!c.is_heap_allocated());
}

macro_rules! format_compact {
    ( $fmt:expr $(, $args:tt)* ) => {
        ToCompactString::to_compact_string(
            &core::format_args!(
                $fmt,
                $(
                    $args,
                )*
            )
        )
    };
}

macro_rules! assert_int_MAX_to_compact_string {
    ($int: ty) => {
        assert_eq!(&*<$int>::MAX.to_string(), &*<$int>::MAX.to_compact_string());
    };
}

#[test]
fn test_to_compact_string() {
    // Test specialisation for bool, char and String
    assert_eq!(&*true.to_string(), "true".to_compact_string());
    assert_eq!(&*false.to_string(), "false".to_compact_string());

    assert_eq!("1", '1'.to_compact_string());
    assert_eq!("2333", "2333".to_string().to_compact_string());
    assert_eq!("2333", "2333".to_compact_string().to_compact_string());

    // Test specialisation for int and nonzero_int using itoa
    assert_eq!("234", 234.to_compact_string());
    assert_eq!(
        "234",
        num::NonZeroU64::new(234).unwrap().to_compact_string()
    );

    assert_int_MAX_to_compact_string!(u8);
    assert_int_MAX_to_compact_string!(i8);

    assert_int_MAX_to_compact_string!(u16);
    assert_int_MAX_to_compact_string!(i16);

    assert_int_MAX_to_compact_string!(u32);
    assert_int_MAX_to_compact_string!(i32);

    assert_int_MAX_to_compact_string!(u64);
    assert_int_MAX_to_compact_string!(i64);

    assert_int_MAX_to_compact_string!(usize);
    assert_int_MAX_to_compact_string!(isize);

    // Test specialisation for f32 and f64 using ryu
    // TODO: Fix bug in powerpc64, which is a little endian system
    #[cfg(not(all(target_arch = "powerpc64", target_pointer_width = "64")))]
    {
        assert_eq!(
            (&*3.2_f32.to_string(), &*288888.290028_f64.to_string()),
            (
                &*3.2_f32.to_compact_string(),
                &*288888.290028_f64.to_compact_string()
            )
        );

        assert_eq!("inf", f32::INFINITY.to_compact_string());
        assert_eq!("-inf", f32::NEG_INFINITY.to_compact_string());

        assert_eq!("inf", f64::INFINITY.to_compact_string());
        assert_eq!("-inf", f64::NEG_INFINITY.to_compact_string());

        assert_eq!("NaN", f32::NAN.to_compact_string());
        assert_eq!("NaN", f64::NAN.to_compact_string());
    }

    // Test generic Display implementation
    assert_eq!("234", "234".to_compact_string());
    assert_eq!("12345", format_compact!("{}", "12345"));
    assert_eq!("112345", format_compact!("1{}", "12345"));
    assert_eq!("1123452", format_compact!("1{}{}", "12345", 2));
    assert_eq!("11234522", format_compact!("1{}{}{}", "12345", 2, '2'));
    assert_eq!(
        "112345221000",
        format_compact!("1{}{}{}{}", "12345", 2, '2', 1000)
    );

    // Test string longer than repr::MAX_SIZE
    assert_eq!(
        "01234567890123456789999999",
        format_compact!("0{}67890123456789{}", "12345", 999999)
    );
}

#[test]
fn test_into_string_large_string_with_excess_capacity() {
    let mut string = String::with_capacity(128);
    string.push_str("abcdefghijklmnopqrstuvwxyz");
    let str_addr = string.as_ptr();
    let str_len = string.len();
    let str_cap = string.capacity();

    let compact = CompactString::from(string);
    let new_string = String::from(compact);
    let new_str_addr = new_string.as_ptr();
    let new_str_len = new_string.len();
    let new_str_cap = new_string.capacity();

    assert_eq!(str_addr, new_str_addr);
    assert_eq!(str_len, new_str_len);
    assert_eq!(str_cap, new_str_cap);
}

#[test]
fn test_into_string_where_32_bit_capacity_is_on_heap() {
    const SIXTEEN_MB: usize = 16 * 1024 * 1024;
    let buf = vec![b'a'; SIXTEEN_MB - 1];
    // SAFETY: `buf` is filled with ASCII `a`s.
    // This primarily speeds up miri, as we don't need to check every byte
    // in the input buffer
    let string = unsafe { String::from_utf8_unchecked(buf) };

    let str_addr = string.as_ptr();
    let str_len = string.len();
    let str_cap = string.capacity();

    let compact = CompactString::from(string);
    let new_string = String::from(compact);
    let new_str_addr = new_string.as_ptr();
    let new_str_len = new_string.len();
    let new_str_cap = new_string.capacity();

    assert_eq!(str_len, new_str_len);

    if cfg!(target_pointer_width = "64") {
        assert_eq!(str_addr, new_str_addr);
        assert_eq!(str_cap, new_str_cap);
    } else {
        assert_eq!(&new_string.as_bytes()[0..10], b"aaaaaaaaaa");
        assert_eq!(str_len, new_str_cap);
    }
}

#[test]
fn test_into_string_small_string_with_excess_capacity() {
    let mut string = String::with_capacity(128);
    string.push_str("abcdef");
    let str_addr = string.as_ptr();
    let str_len = string.len();
    let str_cap = string.capacity();

    let compact = CompactString::from(string);
    let new_string = String::from(compact);
    let new_str_addr = new_string.as_ptr();
    let new_str_len = new_string.len();
    let new_str_cap = new_string.capacity();

    // If small boxed strings are eagerly compacted, the address and capacity assertions won't hold.
    // Compaction is not eager, so these should hold.
    assert_eq!(str_addr, new_str_addr);
    assert_eq!(str_len, new_str_len);
    assert_eq!(str_cap, new_str_cap);
}

#[test]
fn test_into_string_small_string_with_no_excess_capacity() {
    let string = String::from("abcdef");
    let str_addr = string.as_ptr();
    let str_len = string.len();
    let str_cap = string.capacity();

    let compact = CompactString::from(string);
    let new_string = String::from(compact);
    let new_str_addr = new_string.as_ptr();
    let new_str_len = new_string.len();
    let new_str_cap = new_string.capacity();

    // If small boxed strings are eagerly compacted, the address assertion won't hold.
    // Compaction is not eager, so these should hold.
    assert_eq!(str_addr, new_str_addr);
    assert_eq!(str_len, new_str_len);
    assert_eq!(str_cap, new_str_cap);
}

#[test]
fn test_into_string_empty_string() {
    let string = String::new();
    let str_addr = string.as_ptr();
    let str_len = string.len();
    let str_cap = string.capacity();

    let compact = CompactString::from(string);
    let new_string = String::from(compact);
    let new_str_addr = new_string.as_ptr();
    let new_str_len = new_string.len();
    let new_str_cap = new_string.capacity();

    assert_eq!(str_addr, new_str_addr);
    assert_eq!(str_len, new_str_len);
    assert_eq!(str_cap, new_str_cap);
}

#[test]
fn test_into_string_small_str() {
    let data = "abcdef";
    let str_addr = data.as_ptr();
    let str_len = data.len();

    let compact = CompactString::from(data);
    let new_string = String::from(compact);
    let new_str_addr = new_string.as_ptr();
    let new_str_len = new_string.len();
    let new_str_cap = new_string.capacity();

    assert_ne!(str_addr, new_str_addr);
    assert_eq!(str_len, new_str_len);
    assert_eq!(str_len, new_str_cap);
}

#[test]
fn test_into_string_long_str() {
    let data = "abcdefghijklmnopqrstuvwxyz";
    let str_addr = data.as_ptr();
    let str_len = data.len();

    let compact = CompactString::from(data);
    let new_string = String::from(compact);
    let new_str_addr = new_string.as_ptr();
    let new_str_len = new_string.len();
    let new_str_cap = new_string.capacity();

    assert_ne!(str_addr, new_str_addr);
    assert_eq!(str_len, new_str_len);
    assert_eq!(str_len, new_str_cap);
}

#[test]
fn test_into_string_empty_str() {
    let data = "";
    let str_len = data.len();

    let compact = CompactString::from(data);
    let new_string = String::from(compact);
    let new_str_addr = new_string.as_ptr();
    let new_str_len = new_string.len();
    let new_str_cap = new_string.capacity();

    assert_eq!(String::new().as_ptr(), new_str_addr);
    assert_eq!(str_len, new_str_len);
    assert_eq!(str_len, new_str_cap);
}

#[test]
fn test_truncate_noops_if_new_len_greater_than_current() {
    let mut short = CompactString::from("short");
    let short_cap = short.capacity();
    short.truncate(100);

    assert_eq!(short.len(), 5);
    assert_eq!(short.capacity(), short_cap);

    let mut long = CompactString::from("i am a long string that will be allocated on the heap");
    let long_cap = long.capacity();
    long.truncate(500);

    assert_eq!(long.len(), 53);
    assert_eq!(long.capacity(), long_cap);
}

#[test]
#[should_panic(expected = "new_len must lie on char boundary")]
fn test_truncate_panics_on_non_char_boundary() {
    let mut emojis = CompactString::from("😀😀😀😀");
    assert!('😀'.len_utf8() > 1);
    emojis.truncate(1);
}

#[test]
fn test_insert() {
    // insert into empty string
    let mut one_byte = CompactString::from("");
    one_byte.insert(0, '.');
    assert_eq!(one_byte, ".");

    let mut two_bytes = CompactString::from("");
    two_bytes.insert(0, 'Ü');
    assert_eq!(two_bytes, "Ü");

    let mut three_bytes = CompactString::from("");
    three_bytes.insert(0, '€');
    assert_eq!(three_bytes, "€");

    let mut four_bytes = CompactString::from("");
    four_bytes.insert(0, '😀');
    assert_eq!(four_bytes, "😀");

    // insert at the front of string
    let mut one_byte = CompactString::from("😀");
    one_byte.insert(0, '.');
    assert_eq!(one_byte, ".😀");

    let mut two_bytes = CompactString::from("😀");
    two_bytes.insert(0, 'Ü');
    assert_eq!(two_bytes, "Ü😀");

    let mut three_bytes = CompactString::from("😀");
    three_bytes.insert(0, '€');
    assert_eq!(three_bytes, "€😀");

    let mut four_bytes = CompactString::from("😀");
    four_bytes.insert(0, '😀');
    assert_eq!(four_bytes, "😀😀");

    // insert at the end of string
    let mut one_byte = CompactString::from("😀");
    one_byte.insert(4, '.');
    assert_eq!(one_byte, "😀.");

    let mut two_bytes = CompactString::from("😀");
    two_bytes.insert(4, 'Ü');
    assert_eq!(two_bytes, "😀Ü");

    let mut three_bytes = CompactString::from("😀");
    three_bytes.insert(4, '€');
    assert_eq!(three_bytes, "😀€");

    let mut four_bytes = CompactString::from("😀");
    four_bytes.insert(4, '😀');
    assert_eq!(four_bytes, "😀😀");

    // insert in the middle of string
    let mut one_byte = CompactString::from("😀😀");
    one_byte.insert(4, '.');
    assert_eq!(one_byte, "😀.😀");

    let mut two_bytes = CompactString::from("😀😀");
    two_bytes.insert(4, 'Ü');
    assert_eq!(two_bytes, "😀Ü😀");

    let mut three_bytes = CompactString::from("😀😀");
    three_bytes.insert(4, '€');
    assert_eq!(three_bytes, "😀€😀");

    let mut four_bytes = CompactString::from("😀😀");
    four_bytes.insert(4, '😀');
    assert_eq!(four_bytes, "😀😀😀");

    // edge case: new length is 24 bytes
    let mut s = CompactString::from("\u{ffff}\u{ffff}\u{ffff}\u{ffff}\u{ffff}\u{ffff}\u{ffff}");
    s.insert(21, '\u{ffff}');
    assert_eq!(
        s,
        "\u{ffff}\u{ffff}\u{ffff}\u{ffff}\u{ffff}\u{ffff}\u{ffff}\u{ffff}",
    );
}
