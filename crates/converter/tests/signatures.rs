use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::{PublicKey, Signature};
fn decode(s: &str) -> String {
    String::from_utf8(STANDARD.decode(s.trim()).unwrap()).unwrap()
}
#[test]
fn official_tauri_signature_accepts_original_rejects_tampering() {
    // Uses the same verification library as the official updater. This is a
    // crypto boundary test, not a Windows installer end-to-end test.
    let key = PublicKey::decode(&decode(include_str!("fixtures/updater.pub"))).unwrap();
    let sig = Signature::decode(&decode(include_str!("fixtures/payload.sig"))).unwrap();
    let payload = include_bytes!("fixtures/payload.txt");
    key.verify(payload, &sig, true).unwrap();
    assert!(key.verify(b"tampered artifact", &sig, true).is_err());
    assert!(Signature::decode("invalid signature").is_err());
    assert!(sig.trusted_comment().contains("version:0.2.0"));
    let text =
        decode(include_str!("fixtures/payload.sig")).replace("version:0.2.0", "version:9.0.0");
    let changed = Signature::decode(&text).unwrap();
    assert!(key.verify(payload, &changed, true).is_err());
}
