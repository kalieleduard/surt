use super::Slug;

fn slug_of(text: &str) -> Option<String> {
    Slug::derive(text).map(|slug| slug.as_str().to_owned())
}

#[test]
fn a_description_becomes_a_lowercase_hyphenated_slug() {
    assert_eq!(
        slug_of("Add Login with Google / OAuth").as_deref(),
        Some("add-login-with-google-oauth")
    );
    assert_eq!(
        slug_of("  --fix__the  cache!! ").as_deref(),
        Some("fix-the-cache")
    );
}

#[test]
fn accented_latin_letters_are_transliterated_to_ascii() {
    assert_eq!(
        slug_of("Adicionar autenticação à página").as_deref(),
        Some("adicionar-autenticacao-a-pagina")
    );
    assert_eq!(
        slug_of("Straße Œuvre Ærø").as_deref(),
        Some("strasse-oeuvre-aero")
    );
}

#[test]
fn a_long_description_is_cut_at_a_word_boundary_within_40_characters() {
    assert_eq!(
        slug_of("make the login page remember the last user who signed in").as_deref(),
        Some("make-the-login-page-remember-the-last")
    );
    assert_eq!(slug_of(&"x".repeat(50)), Some("x".repeat(40)));
}

#[test]
fn a_description_without_letters_or_digits_has_no_slug() {
    assert_eq!(slug_of("日本語のログイン"), None);
    assert_eq!(slug_of("🚀 / --"), None);
}
