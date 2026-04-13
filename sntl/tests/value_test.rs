use chrono::{TimeZone, Utc};
use sntl::core::types::Value;
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

#[test]
fn value_from_string() {
    let v: Value = "hello".to_string().into();
    assert!(matches!(v, Value::Text(s) if s == "hello"));
}

#[test]
fn value_from_str() {
    let v: Value = Value::from("hello");
    assert!(matches!(v, Value::Text(s) if s == "hello"));
}

#[test]
fn value_from_i64() {
    let v: Value = 42i64.into();
    assert!(matches!(v, Value::BigInt(42)));
}

#[test]
fn value_from_i32() {
    let v: Value = 42i32.into();
    assert!(matches!(v, Value::Int(42)));
}

#[test]
fn value_from_bool() {
    let v: Value = true.into();
    assert!(matches!(v, Value::Bool(true)));
}

#[test]
fn value_from_f64() {
    let v: Value = 2.72f64.into();
    assert!(matches!(v, Value::Double(f) if (f - 2.72).abs() < f64::EPSILON));
}

#[test]
fn value_from_uuid() {
    let id = Uuid::new_v4();
    let v: Value = id.into();
    assert!(matches!(v, Value::Uuid(u) if u == id));
}

#[test]
fn value_from_datetime() {
    let dt = Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap();
    let v: Value = dt.into();
    assert!(matches!(v, Value::Timestamp(d) if d == dt));
}

#[test]
fn value_null() {
    let v = Value::Null;
    assert!(matches!(v, Value::Null));
}

#[test]
fn value_from_option_some() {
    let v: Value = Some(42i64).into();
    assert!(matches!(v, Value::BigInt(42)));
}

#[test]
fn value_from_option_none() {
    let v: Value = Option::<i64>::None.into();
    assert!(matches!(v, Value::Null));
}

#[test]
fn value_from_bytes() {
    let v: Value = vec![0x01u8, 0x02, 0x03].into();
    assert!(matches!(v, Value::Bytes(b) if b == vec![0x01, 0x02, 0x03]));
}

// === New scalar From impls ===

#[test]
fn value_from_i16() {
    let v: Value = 42i16.into();
    assert!(matches!(v, Value::SmallInt(42)));
}

#[test]
fn value_from_f32() {
    let v: Value = 1.5f32.into();
    assert!(matches!(v, Value::Float(f) if (f - 1.5).abs() < f32::EPSILON));
}

#[test]
fn value_from_serde_json() {
    let j = serde_json::json!({"key": "val"});
    let v: Value = j.clone().into();
    assert!(matches!(v, Value::Json(ref inner) if inner == &j));
}

#[test]
fn value_from_ipaddr() {
    let ip: IpAddr = Ipv4Addr::LOCALHOST.into();
    let v: Value = ip.into();
    assert!(matches!(v, Value::Inet(addr) if addr == ip));
}

#[test]
fn value_from_naive_date() {
    let d = chrono::NaiveDate::from_ymd_opt(2026, 4, 13).unwrap();
    let v: Value = d.into();
    assert!(matches!(v, Value::Date(inner) if inner == d));
}

#[test]
fn value_from_naive_time() {
    let t = chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap();
    let v: Value = t.into();
    assert!(matches!(v, Value::Time(inner) if inner == t));
}

#[test]
fn value_from_naive_datetime() {
    let dt = chrono::NaiveDate::from_ymd_opt(2026, 4, 13)
        .unwrap()
        .and_hms_opt(14, 30, 0)
        .unwrap();
    let v: Value = dt.into();
    assert!(matches!(v, Value::TimestampNaive(inner) if inner == dt));
}

#[test]
fn value_from_decimal() {
    let d = rust_decimal::Decimal::new(12345, 2); // 123.45
    let v: Value = d.into();
    assert!(matches!(v, Value::Numeric(inner) if inner == d));
}

// === Complex type constructors ===

#[test]
fn value_interval() {
    let v = Value::Interval(driver::types::interval::PgInterval {
        months: 1,
        days: 2,
        microseconds: 3_000_000,
    });
    assert!(matches!(v, Value::Interval(_)));
}

#[test]
fn value_point() {
    let v = Value::Point(driver::types::geometric::PgPoint { x: 1.0, y: 2.0 });
    assert!(matches!(v, Value::Point(_)));
}

#[test]
fn value_array_homogeneous() {
    let v = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    assert!(matches!(v, Value::Array(ref arr) if arr.len() == 3));
}

#[test]
fn value_macaddr() {
    let v = Value::MacAddr([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    assert!(matches!(v, Value::MacAddr(m) if m == [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]));
}

// === Accessor methods ===

#[test]
fn value_is_methods() {
    assert!(Value::Null.is_null());
    assert!(Value::Bool(true).is_bool());
    assert!(Value::Int(1).is_int());
    assert!(Value::BigInt(1).is_bigint());
    assert!(Value::Double(1.0).is_double());
    assert!(Value::Text("x".into()).is_text());
    assert!(Value::SmallInt(1).is_smallint());
    assert!(Value::Float(1.0).is_float());
    assert!(Value::Json(serde_json::json!(null)).is_json());
    assert!(Value::Date(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()).is_date());
    assert!(Value::Inet(Ipv4Addr::LOCALHOST.into()).is_inet());
    assert!(Value::Array(vec![]).is_array());
    assert!(!Value::Int(1).is_smallint());
    assert!(!Value::SmallInt(1).is_int());
}

#[test]
fn value_as_methods() {
    assert_eq!(Value::SmallInt(42).as_smallint(), Some(42));
    assert_eq!(Value::Int(1).as_smallint(), None);
    assert_eq!(Value::Float(1.5).as_float(), Some(1.5));
    assert_eq!(Value::Int(99).as_int(), Some(99));
    assert_eq!(Value::BigInt(100).as_bigint(), Some(100));
    assert_eq!(Value::Double(2.5).as_double(), Some(2.5));
    assert_eq!(Value::Bool(true).as_bool(), Some(true));
    assert_eq!(Value::Text("hi".into()).as_text(), Some("hi"));
    assert!(Value::Json(serde_json::json!({"a": 1})).as_json().is_some());
    assert_eq!(
        Value::Date(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()).as_date(),
        Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
    );
}

// === Coverage: remaining is_*/as_* for full line coverage ===

#[test]
fn value_is_methods_remaining() {
    assert!(Value::Uuid(uuid::Uuid::nil()).is_uuid());
    assert!(Value::Timestamp(Utc::now()).is_timestamp());
    assert!(Value::Bytes(vec![]).is_bytes());
    assert!(Value::Numeric(rust_decimal::Decimal::ZERO).is_numeric());
    assert!(Value::Money(0).is_money());
    assert!(Value::Xml("<x/>".into()).is_xml());
    assert!(Value::PgLsn(0).is_pglsn());
    assert!(
        Value::Bit(driver::types::bit::PgBit {
            data: vec![],
            bit_length: 0
        })
        .is_bit()
    );
    assert!(Value::Time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()).is_time());
    assert!(
        Value::TimestampNaive(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        )
        .is_timestamp_naive()
    );
    assert!(Value::Cidr(Ipv4Addr::LOCALHOST.into()).is_cidr());
    assert!(Value::MacAddr([0; 6]).is_macaddr());
    assert!(
        Value::Interval(driver::types::interval::PgInterval {
            months: 0,
            days: 0,
            microseconds: 0
        })
        .is_interval()
    );
    assert!(Value::Point(driver::types::geometric::PgPoint { x: 0.0, y: 0.0 }).is_point());
    assert!(
        Value::Line(driver::types::geometric::PgLine {
            a: 0.0,
            b: 0.0,
            c: 0.0
        })
        .is_line()
    );
    assert!(
        Value::LineSegment(driver::types::geometric::PgLSeg {
            start: driver::types::geometric::PgPoint { x: 0.0, y: 0.0 },
            end: driver::types::geometric::PgPoint { x: 1.0, y: 1.0 },
        })
        .is_line_segment()
    );
    assert!(
        Value::Box(driver::types::geometric::PgBox {
            upper_right: driver::types::geometric::PgPoint { x: 1.0, y: 1.0 },
            lower_left: driver::types::geometric::PgPoint { x: 0.0, y: 0.0 },
        })
        .is_box()
    );
    assert!(
        Value::Circle(driver::types::geometric::PgCircle {
            center: driver::types::geometric::PgPoint { x: 0.0, y: 0.0 },
            radius: 1.0,
        })
        .is_circle()
    );
    assert!(!Value::Int(1).is_custom());
}

#[test]
fn value_as_methods_remaining() {
    assert_eq!(
        Value::Uuid(uuid::Uuid::nil()).as_uuid(),
        Some(uuid::Uuid::nil())
    );
    assert!(Value::Timestamp(Utc::now()).as_timestamp().is_some());
    assert_eq!(Value::Bytes(vec![1, 2]).as_bytes(), Some(&[1u8, 2][..]));
    assert_eq!(
        Value::Numeric(rust_decimal::Decimal::ONE).as_numeric(),
        Some(rust_decimal::Decimal::ONE)
    );
    assert_eq!(Value::Money(100).as_money(), Some(100));
    let t = chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap();
    assert_eq!(Value::Time(t).as_time(), Some(t));
    let dt = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    assert_eq!(Value::TimestampNaive(dt).as_timestamp_naive(), Some(dt));
    assert_eq!(
        Value::Inet(Ipv4Addr::LOCALHOST.into()).as_inet(),
        Some(Ipv4Addr::LOCALHOST.into())
    );
    assert!(
        Value::Interval(driver::types::interval::PgInterval {
            months: 0,
            days: 0,
            microseconds: 0
        })
        .as_interval()
        .is_some()
    );
    let pt = driver::types::geometric::PgPoint { x: 1.0, y: 2.0 };
    assert_eq!(Value::Point(pt).as_point(), Some(pt));
    assert_eq!(
        Value::Array(vec![Value::Int(1)]).as_array(),
        Some(&[Value::Int(1)][..])
    );
    // None cases
    assert_eq!(Value::Text("x".into()).as_int(), None);
    assert_eq!(Value::Int(1).as_uuid(), None);
    assert_eq!(Value::Int(1).as_timestamp(), None);
    assert_eq!(Value::Int(1).as_bytes(), None);
    assert_eq!(Value::Int(1).as_numeric(), None);
    assert_eq!(Value::Int(1).as_money(), None);
    assert_eq!(Value::Int(1).as_json(), None);
    assert_eq!(Value::Int(1).as_time(), None);
    assert_eq!(Value::Int(1).as_timestamp_naive(), None);
    assert_eq!(Value::Int(1).as_inet(), None);
    assert!(Value::Int(1).as_interval().is_none());
    assert!(Value::Int(1).as_point().is_none());
    assert!(Value::Int(1).as_array().is_none());
}

// === Coverage: Debug, Display, PartialEq for all variants ===

#[test]
fn value_debug_all_variants() {
    use driver::types::geometric::*;
    use driver::types::interval::PgInterval;

    let values: Vec<Value> = vec![
        Value::Null,
        Value::Bool(true),
        Value::Int(1),
        Value::BigInt(1),
        Value::Double(1.0),
        Value::Text("t".into()),
        Value::Uuid(uuid::Uuid::nil()),
        Value::Timestamp(Utc::now()),
        Value::Bytes(vec![1]),
        Value::SmallInt(1),
        Value::Float(1.0),
        Value::Numeric(rust_decimal::Decimal::ONE),
        Value::Money(100),
        Value::Xml("<x/>".into()),
        Value::PgLsn(0x0000_0001_0000_0002),
        Value::Bit(driver::types::bit::PgBit {
            data: vec![0xFF],
            bit_length: 8,
        }),
        Value::Json(serde_json::json!({"k": "v"})),
        Value::Date(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
        Value::Time(chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
        Value::TimestampNaive(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        ),
        Value::Inet(Ipv4Addr::LOCALHOST.into()),
        Value::Cidr(Ipv4Addr::LOCALHOST.into()),
        Value::MacAddr([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]),
        Value::Interval(PgInterval {
            months: 1,
            days: 2,
            microseconds: 3,
        }),
        Value::Point(PgPoint { x: 1.0, y: 2.0 }),
        Value::Line(PgLine {
            a: 1.0,
            b: 2.0,
            c: 3.0,
        }),
        Value::LineSegment(PgLSeg {
            start: PgPoint { x: 0.0, y: 0.0 },
            end: PgPoint { x: 1.0, y: 1.0 },
        }),
        Value::Box(PgBox {
            upper_right: PgPoint { x: 1.0, y: 1.0 },
            lower_left: PgPoint { x: 0.0, y: 0.0 },
        }),
        Value::Circle(PgCircle {
            center: PgPoint { x: 0.0, y: 0.0 },
            radius: 5.0,
        }),
        Value::Array(vec![Value::Int(1)]),
    ];
    for v in &values {
        let _ = format!("{v:?}");
        let _ = format!("{v}");
    }
}

#[test]
fn value_display_custom() {
    use std::sync::Arc;
    let custom = Value::Custom(Arc::new(42i32));
    assert_eq!(format!("{custom}"), "<custom>");
    assert_eq!(format!("{custom:?}"), "Custom(\"<opaque>\")");
}

#[test]
fn value_partial_eq_all_variants() {
    use driver::types::geometric::*;
    use driver::types::interval::PgInterval;
    use driver::types::range::{PgRange, RangeBound};

    // Same-variant equality
    assert_eq!(Value::Null, Value::Null);
    assert_eq!(Value::Money(100), Value::Money(100));
    assert_eq!(Value::Xml("<x/>".into()), Value::Xml("<x/>".into()));
    assert_eq!(Value::PgLsn(42), Value::PgLsn(42));
    assert_eq!(
        Value::Bit(driver::types::bit::PgBit {
            data: vec![0xFF],
            bit_length: 8
        }),
        Value::Bit(driver::types::bit::PgBit {
            data: vec![0xFF],
            bit_length: 8
        })
    );
    assert_eq!(
        Value::Cidr(Ipv4Addr::LOCALHOST.into()),
        Value::Cidr(Ipv4Addr::LOCALHOST.into())
    );
    assert_eq!(Value::MacAddr([1; 6]), Value::MacAddr([1; 6]));
    assert_eq!(
        Value::Interval(PgInterval {
            months: 1,
            days: 2,
            microseconds: 3
        }),
        Value::Interval(PgInterval {
            months: 1,
            days: 2,
            microseconds: 3
        })
    );
    assert_eq!(
        Value::Line(PgLine {
            a: 1.0,
            b: 2.0,
            c: 3.0
        }),
        Value::Line(PgLine {
            a: 1.0,
            b: 2.0,
            c: 3.0
        })
    );
    assert_eq!(
        Value::LineSegment(PgLSeg {
            start: PgPoint { x: 0.0, y: 0.0 },
            end: PgPoint { x: 1.0, y: 1.0 }
        }),
        Value::LineSegment(PgLSeg {
            start: PgPoint { x: 0.0, y: 0.0 },
            end: PgPoint { x: 1.0, y: 1.0 }
        })
    );
    assert_eq!(
        Value::Box(PgBox {
            upper_right: PgPoint { x: 1.0, y: 1.0 },
            lower_left: PgPoint { x: 0.0, y: 0.0 }
        }),
        Value::Box(PgBox {
            upper_right: PgPoint { x: 1.0, y: 1.0 },
            lower_left: PgPoint { x: 0.0, y: 0.0 }
        })
    );
    assert_eq!(
        Value::Circle(PgCircle {
            center: PgPoint { x: 0.0, y: 0.0 },
            radius: 5.0
        }),
        Value::Circle(PgCircle {
            center: PgPoint { x: 0.0, y: 0.0 },
            radius: 5.0
        })
    );

    // Range equality
    let r = PgRange {
        lower: RangeBound::Inclusive(1i32),
        upper: RangeBound::Exclusive(10),
        is_empty: false,
        range_oid: sntl::driver::Oid::INT4RANGE,
        element_oid: sntl::driver::Oid::INT4,
    };
    assert_eq!(Value::Int4Range(r.clone()), Value::Int4Range(r));

    // Cross-variant inequality
    assert_ne!(Value::Int(1), Value::SmallInt(1));
    assert_ne!(Value::Int(1), Value::BigInt(1));

    // Custom never equal
    use std::sync::Arc;
    assert_ne!(Value::Custom(Arc::new(1i32)), Value::Custom(Arc::new(1i32)));
}

// === Coverage: ToSql for remaining variants ===

#[test]
fn value_tosql_xml() {
    use sntl::driver::types::ToSql;
    let v = Value::Xml("<root/>".into());
    assert_eq!(v.oid(), sntl::driver::Oid::XML);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.as_ref(), b"<root/>");
}

#[test]
fn value_tosql_pglsn() {
    use sntl::driver::types::ToSql;
    let v = Value::PgLsn(0x0000_0001_0000_0002);
    assert_eq!(v.oid(), sntl::driver::Oid::PG_LSN);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 8);
}

#[test]
fn value_tosql_numeric() {
    use sntl::driver::types::ToSql;
    let v = Value::Numeric(rust_decimal::Decimal::new(12345, 2));
    assert_eq!(v.oid(), sntl::driver::Oid::NUMERIC);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(!buf.is_empty());
}

#[test]
fn value_tosql_cidr() {
    use sntl::driver::types::ToSql;
    let v = Value::Cidr(Ipv4Addr::LOCALHOST.into());
    assert_eq!(v.oid(), sntl::driver::Oid::CIDR);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(!buf.is_empty());
}

#[test]
fn value_tosql_bit() {
    use sntl::driver::types::ToSql;
    let v = Value::Bit(driver::types::bit::PgBit {
        data: vec![0xFF],
        bit_length: 8,
    });
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(!buf.is_empty());
}

#[test]
fn value_tosql_line() {
    use sntl::driver::types::ToSql;
    let v = Value::Line(driver::types::geometric::PgLine {
        a: 1.0,
        b: 2.0,
        c: 3.0,
    });
    assert_eq!(v.oid(), sntl::driver::Oid::LINE);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 24);
}

#[test]
fn value_tosql_lseg() {
    use driver::types::geometric::*;
    use sntl::driver::types::ToSql;
    let v = Value::LineSegment(PgLSeg {
        start: PgPoint { x: 0.0, y: 0.0 },
        end: PgPoint { x: 1.0, y: 1.0 },
    });
    assert_eq!(v.oid(), sntl::driver::Oid::LSEG);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 32);
}

#[test]
fn value_tosql_box() {
    use driver::types::geometric::*;
    use sntl::driver::types::ToSql;
    let v = Value::Box(PgBox {
        upper_right: PgPoint { x: 1.0, y: 1.0 },
        lower_left: PgPoint { x: 0.0, y: 0.0 },
    });
    assert_eq!(v.oid(), sntl::driver::Oid::PG_BOX);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 32);
}

#[test]
fn value_tosql_circle() {
    use driver::types::geometric::*;
    use sntl::driver::types::ToSql;
    let v = Value::Circle(PgCircle {
        center: PgPoint { x: 0.0, y: 0.0 },
        radius: 5.0,
    });
    assert_eq!(v.oid(), sntl::driver::Oid::CIRCLE);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 24);
}

#[test]
fn value_tosql_ranges() {
    use driver::types::range::{PgRange, RangeBound};
    use sntl::driver::types::ToSql;

    let r = PgRange {
        lower: RangeBound::Inclusive(1i32),
        upper: RangeBound::Exclusive(10),
        is_empty: false,
        range_oid: sntl::driver::Oid::INT4RANGE,
        element_oid: sntl::driver::Oid::INT4,
    };
    let v = Value::Int4Range(r);
    assert_eq!(v.oid(), sntl::driver::Oid::INT4RANGE);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(!buf.is_empty());

    let r8 = PgRange {
        lower: RangeBound::Inclusive(1i64),
        upper: RangeBound::Exclusive(10),
        is_empty: false,
        range_oid: sntl::driver::Oid::INT8RANGE,
        element_oid: sntl::driver::Oid::INT8,
    };
    let v = Value::Int8Range(r8);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(!buf.is_empty());

    let rn = PgRange {
        lower: RangeBound::Inclusive(rust_decimal::Decimal::ONE),
        upper: RangeBound::Exclusive(rust_decimal::Decimal::new(10, 0)),
        is_empty: false,
        range_oid: sntl::driver::Oid::NUMRANGE,
        element_oid: sntl::driver::Oid::NUMERIC,
    };
    let v = Value::NumRange(rn);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(!buf.is_empty());

    let d1 = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let d2 = chrono::NaiveDate::from_ymd_opt(2026, 12, 31)
        .unwrap()
        .and_hms_opt(23, 59, 59)
        .unwrap();
    let rt = PgRange {
        lower: RangeBound::Inclusive(d1),
        upper: RangeBound::Exclusive(d2),
        is_empty: false,
        range_oid: sntl::driver::Oid::TSRANGE,
        element_oid: sntl::driver::Oid::TIMESTAMP,
    };
    let v = Value::TsRange(rt);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(!buf.is_empty());

    let rtz = PgRange {
        lower: RangeBound::Inclusive(Utc::now()),
        upper: RangeBound::Unbounded,
        is_empty: false,
        range_oid: sntl::driver::Oid::TSTZRANGE,
        element_oid: sntl::driver::Oid::TIMESTAMPTZ,
    };
    let v = Value::TsTzRange(rtz);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(!buf.is_empty());

    let rd = PgRange {
        lower: RangeBound::Inclusive(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
        upper: RangeBound::Exclusive(chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()),
        is_empty: false,
        range_oid: sntl::driver::Oid::DATERANGE,
        element_oid: sntl::driver::Oid::DATE,
    };
    let v = Value::DateRange(rd);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(!buf.is_empty());
}

#[test]
fn value_tosql_custom() {
    use sntl::driver::types::ToSql;
    use std::sync::Arc;
    let v = Value::Custom(Arc::new(42i32));
    assert_eq!(v.oid(), sntl::driver::Oid::INT4);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.as_ref(), &42i32.to_be_bytes());
}

#[test]
fn value_tosql_timestamp_naive() {
    use sntl::driver::types::ToSql;
    let dt = chrono::NaiveDate::from_ymd_opt(2026, 4, 13)
        .unwrap()
        .and_hms_opt(14, 30, 0)
        .unwrap();
    let v = Value::TimestampNaive(dt);
    assert_eq!(v.oid(), sntl::driver::Oid::TIMESTAMP);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert_eq!(buf.len(), 8);
}

#[test]
fn value_display_ranges() {
    use driver::types::range::{PgRange, RangeBound};
    let r = PgRange {
        lower: RangeBound::Inclusive(1i32),
        upper: RangeBound::Exclusive(10),
        is_empty: false,
        range_oid: sntl::driver::Oid::INT4RANGE,
        element_oid: sntl::driver::Oid::INT4,
    };
    let _ = format!("{}", Value::Int4Range(r.clone()));
    let _ = format!(
        "{}",
        Value::Int8Range(PgRange {
            lower: RangeBound::Inclusive(1i64),
            upper: RangeBound::Exclusive(10),
            is_empty: false,
            range_oid: sntl::driver::Oid::INT8RANGE,
            element_oid: sntl::driver::Oid::INT8,
        })
    );
    let _ = format!(
        "{}",
        Value::NumRange(PgRange {
            lower: RangeBound::Inclusive(rust_decimal::Decimal::ONE),
            upper: RangeBound::Unbounded,
            is_empty: false,
            range_oid: sntl::driver::Oid::NUMRANGE,
            element_oid: sntl::driver::Oid::NUMERIC,
        })
    );
    let _ = format!(
        "{}",
        Value::TsRange(PgRange {
            lower: RangeBound::Unbounded,
            upper: RangeBound::Unbounded,
            is_empty: true,
            range_oid: sntl::driver::Oid::TSRANGE,
            element_oid: sntl::driver::Oid::TIMESTAMP,
        })
    );
    let _ = format!(
        "{}",
        Value::TsTzRange(PgRange {
            lower: RangeBound::Unbounded,
            upper: RangeBound::Unbounded,
            is_empty: true,
            range_oid: sntl::driver::Oid::TSTZRANGE,
            element_oid: sntl::driver::Oid::TIMESTAMPTZ,
        })
    );
    let _ = format!(
        "{}",
        Value::DateRange(PgRange {
            lower: RangeBound::Unbounded,
            upper: RangeBound::Unbounded,
            is_empty: true,
            range_oid: sntl::driver::Oid::DATERANGE,
            element_oid: sntl::driver::Oid::DATE,
        })
    );
}

#[test]
fn value_array_with_null_element() {
    use sntl::driver::types::ToSql;
    let v = Value::Array(vec![Value::Int(1), Value::Null, Value::Int(3)]);
    let mut buf = bytes::BytesMut::new();
    v.to_sql(&mut buf).unwrap();
    assert!(!buf.is_empty());
}

// === Coverage: Debug/PartialEq for range variants ===

#[test]
fn value_debug_and_display_ranges() {
    use driver::types::range::{PgRange, RangeBound};
    let ranges: Vec<Value> = vec![
        Value::Int4Range(PgRange {
            lower: RangeBound::Inclusive(1i32),
            upper: RangeBound::Exclusive(10),
            is_empty: false,
            range_oid: sntl::driver::Oid::INT4RANGE,
            element_oid: sntl::driver::Oid::INT4,
        }),
        Value::Int8Range(PgRange {
            lower: RangeBound::Inclusive(1i64),
            upper: RangeBound::Exclusive(10),
            is_empty: false,
            range_oid: sntl::driver::Oid::INT8RANGE,
            element_oid: sntl::driver::Oid::INT8,
        }),
        Value::NumRange(PgRange {
            lower: RangeBound::Inclusive(rust_decimal::Decimal::ONE),
            upper: RangeBound::Unbounded,
            is_empty: false,
            range_oid: sntl::driver::Oid::NUMRANGE,
            element_oid: sntl::driver::Oid::NUMERIC,
        }),
        Value::TsRange(PgRange {
            lower: RangeBound::Unbounded,
            upper: RangeBound::Unbounded,
            is_empty: true,
            range_oid: sntl::driver::Oid::TSRANGE,
            element_oid: sntl::driver::Oid::TIMESTAMP,
        }),
        Value::TsTzRange(PgRange {
            lower: RangeBound::Unbounded,
            upper: RangeBound::Unbounded,
            is_empty: true,
            range_oid: sntl::driver::Oid::TSTZRANGE,
            element_oid: sntl::driver::Oid::TIMESTAMPTZ,
        }),
        Value::DateRange(PgRange {
            lower: RangeBound::Unbounded,
            upper: RangeBound::Unbounded,
            is_empty: true,
            range_oid: sntl::driver::Oid::DATERANGE,
            element_oid: sntl::driver::Oid::DATE,
        }),
    ];
    for v in &ranges {
        let _ = format!("{v:?}"); // exercises Debug
    }
}

#[test]
fn value_partial_eq_range_variants() {
    use driver::types::range::{PgRange, RangeBound};

    let r8 = PgRange {
        lower: RangeBound::Inclusive(1i64),
        upper: RangeBound::Exclusive(10),
        is_empty: false,
        range_oid: sntl::driver::Oid::INT8RANGE,
        element_oid: sntl::driver::Oid::INT8,
    };
    assert_eq!(Value::Int8Range(r8.clone()), Value::Int8Range(r8));

    let rn = PgRange {
        lower: RangeBound::Inclusive(rust_decimal::Decimal::ONE),
        upper: RangeBound::Unbounded,
        is_empty: false,
        range_oid: sntl::driver::Oid::NUMRANGE,
        element_oid: sntl::driver::Oid::NUMERIC,
    };
    assert_eq!(Value::NumRange(rn.clone()), Value::NumRange(rn));

    let rt = PgRange {
        lower: RangeBound::Unbounded,
        upper: RangeBound::Unbounded,
        is_empty: true,
        range_oid: sntl::driver::Oid::TSRANGE,
        element_oid: sntl::driver::Oid::TIMESTAMP,
    };
    assert_eq!(Value::TsRange(rt.clone()), Value::TsRange(rt));

    let rtz = PgRange {
        lower: RangeBound::Unbounded,
        upper: RangeBound::Unbounded,
        is_empty: true,
        range_oid: sntl::driver::Oid::TSTZRANGE,
        element_oid: sntl::driver::Oid::TIMESTAMPTZ,
    };
    assert_eq!(Value::TsTzRange(rtz.clone()), Value::TsTzRange(rtz));

    let rd = PgRange {
        lower: RangeBound::Unbounded,
        upper: RangeBound::Unbounded,
        is_empty: true,
        range_oid: sntl::driver::Oid::DATERANGE,
        element_oid: sntl::driver::Oid::DATE,
    };
    assert_eq!(Value::DateRange(rd.clone()), Value::DateRange(rd));
}

// === Coverage: PartialEq remaining branches ===

#[test]
fn value_partial_eq_remaining() {
    assert_eq!(
        Value::Json(serde_json::json!(1)),
        Value::Json(serde_json::json!(1))
    );
    assert_eq!(
        Value::TimestampNaive(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        ),
        Value::TimestampNaive(
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
        )
    );
    let pt = driver::types::geometric::PgPoint { x: 1.0, y: 2.0 };
    assert_eq!(Value::Point(pt), Value::Point(pt));
}

#[test]
fn value_partial_eq_basic_variants() {
    // Exercises PartialEq branches for basic types that existing tests
    // only cover via matches! (which bypasses our manual PartialEq impl)
    assert_eq!(Value::Bool(true), Value::Bool(true));
    assert_eq!(Value::Int(42), Value::Int(42));
    assert_eq!(Value::BigInt(100), Value::BigInt(100));
    assert_eq!(Value::Double(1.5), Value::Double(1.5));
    assert_eq!(Value::Text("hi".into()), Value::Text("hi".into()));
    assert_eq!(
        Value::Uuid(uuid::Uuid::nil()),
        Value::Uuid(uuid::Uuid::nil())
    );
    let ts = Utc::now();
    assert_eq!(Value::Timestamp(ts), Value::Timestamp(ts));
    assert_eq!(Value::Bytes(vec![1, 2]), Value::Bytes(vec![1, 2]));
    assert_eq!(Value::SmallInt(1), Value::SmallInt(1));
    assert_eq!(Value::Float(1.0), Value::Float(1.0));
    assert_eq!(
        Value::Numeric(rust_decimal::Decimal::ONE),
        Value::Numeric(rust_decimal::Decimal::ONE)
    );
    let d = chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    assert_eq!(Value::Date(d), Value::Date(d));
    let t = chrono::NaiveTime::from_hms_opt(12, 0, 0).unwrap();
    assert_eq!(Value::Time(t), Value::Time(t));
    assert_eq!(
        Value::Inet(Ipv4Addr::LOCALHOST.into()),
        Value::Inet(Ipv4Addr::LOCALHOST.into())
    );
    assert_eq!(
        Value::Array(vec![Value::Int(1)]),
        Value::Array(vec![Value::Int(1)])
    );
}

// === Coverage: array_oid for various element types ===

#[test]
fn value_array_oid_all_types() {
    use sntl::driver::Oid;
    use sntl::driver::types::ToSql;

    assert_eq!(Value::Array(vec![Value::Bool(true)]).oid(), Oid::BOOL_ARRAY);
    assert_eq!(
        Value::Array(vec![Value::SmallInt(1)]).oid(),
        Oid::INT2_ARRAY
    );
    assert_eq!(Value::Array(vec![Value::Int(1)]).oid(), Oid::INT4_ARRAY);
    assert_eq!(Value::Array(vec![Value::BigInt(1)]).oid(), Oid::INT8_ARRAY);
    assert_eq!(
        Value::Array(vec![Value::Float(1.0)]).oid(),
        Oid::FLOAT4_ARRAY
    );
    assert_eq!(
        Value::Array(vec![Value::Double(1.0)]).oid(),
        Oid::FLOAT8_ARRAY
    );
    assert_eq!(
        Value::Array(vec![Value::Text("a".into())]).oid(),
        Oid::TEXT_ARRAY
    );
    assert_eq!(
        Value::Array(vec![Value::Uuid(uuid::Uuid::nil())]).oid(),
        Oid::UUID_ARRAY
    );
    assert_eq!(
        Value::Array(vec![Value::Numeric(rust_decimal::Decimal::ONE)]).oid(),
        Oid::NUMERIC_ARRAY
    );
    assert_eq!(
        Value::Array(vec![Value::Inet(Ipv4Addr::LOCALHOST.into())]).oid(),
        Oid::INET_ARRAY
    );
    assert_eq!(
        Value::Array(vec![Value::Interval(driver::types::interval::PgInterval {
            months: 0,
            days: 0,
            microseconds: 0,
        })])
        .oid(),
        Oid::INTERVAL_ARRAY
    );
}

#[test]
fn value_array_unsupported_element_oid_fallback() {
    use sntl::driver::types::ToSql;
    // Point doesn't have a mapped array OID — oid() falls back to TEXT_ARRAY
    let v = Value::Array(vec![Value::Point(driver::types::geometric::PgPoint {
        x: 0.0,
        y: 0.0,
    })]);
    assert_eq!(v.oid(), sntl::driver::Oid::TEXT_ARRAY);
}

#[test]
fn value_array_all_null_errors() {
    use sntl::driver::types::ToSql;
    let v = Value::Array(vec![Value::Null, Value::Null]);
    let mut buf = bytes::BytesMut::new();
    assert!(v.to_sql(&mut buf).is_err());
}

// === Coverage: ToSql is_null returns false for non-null ===

#[test]
fn value_tosql_is_null_trait_method() {
    use sntl::driver::types::ToSql;
    assert!(Value::Null.is_null());
    assert!(!Value::Int(0).is_null());
    assert!(!Value::SmallInt(0).is_null());
}

// === Coverage: oid() for Bit variant (delegates to inner) ===

#[test]
fn value_bit_oid() {
    use sntl::driver::types::ToSql;
    let v = Value::Bit(driver::types::bit::PgBit {
        data: vec![0xFF],
        bit_length: 8,
    });
    let _ = v.oid(); // exercises L295
}

// === Coverage: as_* None branches and Display multi-element array ===

#[test]
fn value_as_none_branches() {
    assert_eq!(Value::Int(1).as_bool(), None);
    assert_eq!(Value::Bool(true).as_double(), None);
    assert_eq!(Value::Bool(true).as_text(), None);
    assert_eq!(Value::Bool(true).as_float(), None);
    assert_eq!(Value::Bool(true).as_date(), None);
    assert_eq!(Value::Bool(true).as_bigint(), None);
}

#[test]
fn value_tosql_is_null_trait() {
    // Exercise the ToSql::is_null() method through trait object dispatch
    let null_val = Value::Null;
    let int_val = Value::Int(1);
    let vals: Vec<&dyn sntl::driver::ToSql> = vec![&null_val, &int_val];
    assert!(vals[0].is_null());
    assert!(!vals[1].is_null());
}

#[test]
fn value_display_multi_element_array() {
    let v = Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    assert_eq!(format!("{v}"), "{1,2,3}");
}

#[test]
fn value_oid_range_variants() {
    use driver::types::range::{PgRange, RangeBound};
    use sntl::driver::types::ToSql;

    let _ = Value::Int4Range(PgRange {
        lower: RangeBound::Inclusive(1i32),
        upper: RangeBound::Exclusive(10),
        is_empty: false,
        range_oid: sntl::driver::Oid::INT4RANGE,
        element_oid: sntl::driver::Oid::INT4,
    })
    .oid();

    let _ = Value::Int8Range(PgRange {
        lower: RangeBound::Inclusive(1i64),
        upper: RangeBound::Exclusive(10),
        is_empty: false,
        range_oid: sntl::driver::Oid::INT8RANGE,
        element_oid: sntl::driver::Oid::INT8,
    })
    .oid();

    let _ = Value::NumRange(PgRange {
        lower: RangeBound::Inclusive(rust_decimal::Decimal::ONE),
        upper: RangeBound::Unbounded,
        is_empty: false,
        range_oid: sntl::driver::Oid::NUMRANGE,
        element_oid: sntl::driver::Oid::NUMERIC,
    })
    .oid();

    let _ = Value::TsRange(PgRange {
        lower: RangeBound::Unbounded,
        upper: RangeBound::Unbounded,
        is_empty: true,
        range_oid: sntl::driver::Oid::TSRANGE,
        element_oid: sntl::driver::Oid::TIMESTAMP,
    })
    .oid();

    let _ = Value::TsTzRange(PgRange {
        lower: RangeBound::Unbounded,
        upper: RangeBound::Unbounded,
        is_empty: true,
        range_oid: sntl::driver::Oid::TSTZRANGE,
        element_oid: sntl::driver::Oid::TIMESTAMPTZ,
    })
    .oid();

    let _ = Value::DateRange(PgRange {
        lower: RangeBound::Unbounded,
        upper: RangeBound::Unbounded,
        is_empty: true,
        range_oid: sntl::driver::Oid::DATERANGE,
        element_oid: sntl::driver::Oid::DATE,
    })
    .oid();
}
