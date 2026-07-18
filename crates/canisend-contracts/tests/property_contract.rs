//! Deterministic generated property tests for public strong primitives.

use canisend_contracts::{EntityId, Revision, SafeRelativePath, Sha256Digest};

const GENERATED_CASES: usize = 512;

fn next_state(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

#[test]
fn property_generated_portable_paths_round_trip_without_normalization() {
    let mut state = 0x4341_4e49_5345_4e44_u64;
    for case in 0..GENERATED_CASES {
        let depth = (next_state(&mut state) % 6 + 1) as usize;
        let segments = (0..depth)
            .map(|index| {
                let nonce = next_state(&mut state);
                if (case + index) % 11 == 0 {
                    format!("résumé-東京-{case:03x}-{index}-{nonce:016x}")
                } else {
                    format!("segment-{case:03x}-{index}-{nonce:016x}")
                }
            })
            .collect::<Vec<_>>();
        let source = segments.join("/");
        let path = SafeRelativePath::try_new(source.clone()).expect("generated portable path");
        assert_eq!(path.as_str(), source);

        let encoded = serde_json::to_string(&path).expect("serialize generated path");
        let decoded: SafeRelativePath =
            serde_json::from_str(&encoded).expect("deserialize generated path");
        assert_eq!(decoded, path);
    }
}

#[test]
fn property_inserting_any_reserved_component_is_always_rejected() {
    let reserved = [
        "",
        ".",
        "..",
        "CON",
        "aux.txt",
        "LPT9.log",
        "trailing.",
        "trailing ",
    ];
    let mut state = 0x5041_5448_5341_4645_u64;
    for case in 0..GENERATED_CASES / 4 {
        let left = format!("left-{case:02x}-{:016x}", next_state(&mut state));
        let right = format!("right-{case:02x}-{:016x}", next_state(&mut state));
        for component in reserved {
            for parts in [
                vec![component, left.as_str()],
                vec![left.as_str(), component, right.as_str()],
                vec![right.as_str(), component],
            ] {
                let candidate = parts.join("/");
                assert!(
                    SafeRelativePath::try_new(&candidate).is_err(),
                    "accepted generated reserved path `{candidate}`"
                );
            }
        }
        for candidate in [
            format!(".canisend/{left}"),
            format!("/{left}"),
            format!("{left}\\{right}"),
            format!("{left}:{right}"),
            format!("{left}/line\nbreak"),
        ] {
            assert!(
                SafeRelativePath::try_new(&candidate).is_err(),
                "accepted generated unsafe path `{candidate}`"
            );
        }
    }
}

#[test]
fn property_generated_sha256_digests_round_trip_and_mutations_fail() {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut state = 0x4449_4745_5354_5632_u64;
    for _ in 0..GENERATED_CASES {
        let source = (0..64)
            .map(|_| HEX[(next_state(&mut state) & 0x0f) as usize] as char)
            .collect::<String>();
        let digest = Sha256Digest::try_new(source.clone()).expect("generated SHA-256 digest");
        let encoded = serde_json::to_string(&digest).expect("serialize generated digest");
        let decoded: Sha256Digest =
            serde_json::from_str(&encoded).expect("deserialize generated digest");
        assert_eq!(decoded, digest);

        assert!(Sha256Digest::try_new(&source[..63]).is_err());
        assert!(Sha256Digest::try_new(format!("{source}0")).is_err());
        let mut uppercase = source.into_bytes();
        uppercase[0] = b'A';
        let uppercase = String::from_utf8(uppercase).expect("ASCII digest mutation");
        assert!(Sha256Digest::try_new(uppercase).is_err());
    }
}

#[test]
fn property_generated_uuidv7_and_revisions_preserve_identity() {
    let mut state = 0x5555_4944_5637_0002_u64;
    let variants = ['8', '9', 'a', 'b'];
    for case in 1..=GENERATED_CASES {
        let value = next_state(&mut state);
        let source = format!(
            "{:08x}-{:04x}-7{:03x}-{}{:03x}-{:012x}",
            (value >> 32) as u32,
            (value >> 16) as u16,
            value as u16 & 0x0fff,
            variants[case % variants.len()],
            (value >> 20) as u16 & 0x0fff,
            value & 0x0000_ffff_ffff_ffff
        );
        let id = EntityId::try_new(source.clone()).expect("generated UUIDv7");
        let encoded = serde_json::to_string(&id).expect("serialize generated UUIDv7");
        let decoded: EntityId =
            serde_json::from_str(&encoded).expect("deserialize generated UUIDv7");
        assert_eq!(decoded.as_str(), source);

        let revision = Revision::try_new(case as u64).expect("generated positive revision");
        let encoded = serde_json::to_string(&revision).expect("serialize generated revision");
        let decoded: Revision =
            serde_json::from_str(&encoded).expect("deserialize generated revision");
        assert_eq!(decoded, revision);
    }
    assert!(Revision::try_new(0).is_err());
}
