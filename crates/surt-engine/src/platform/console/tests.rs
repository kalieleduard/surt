#[cfg(unix)]
mod locale {
    use super::super::native::locale_is_utf8;

    fn environment<'a>(set: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            set.iter()
                .find(|(variable, _)| *variable == name)
                .map(|(_, value)| (*value).to_owned())
        }
    }

    #[test]
    fn a_locale_naming_utf8_in_any_spelling_outputs_utf8() {
        for locale in ["en_US.UTF-8", "C.UTF-8", "pt_BR.utf8", "en_GB.Utf-8@euro"] {
            assert!(locale_is_utf8(environment(&[("LANG", locale)])), "{locale}");
        }
    }

    #[test]
    fn a_locale_not_naming_utf8_outputs_ascii() {
        for locale in ["C", "POSIX", "en_US.ISO-8859-1", "pt_BR"] {
            assert!(
                !locale_is_utf8(environment(&[("LANG", locale)])),
                "{locale}"
            );
        }
    }

    #[test]
    fn the_first_set_variable_decides_in_lc_all_lc_ctype_lang_order() {
        assert!(!locale_is_utf8(environment(&[
            ("LC_ALL", "C"),
            ("LC_CTYPE", "en_US.UTF-8"),
            ("LANG", "en_US.UTF-8"),
        ])));
        assert!(locale_is_utf8(environment(&[
            ("LC_CTYPE", "en_US.UTF-8"),
            ("LANG", "C"),
        ])));
        assert!(!locale_is_utf8(environment(&[
            ("LC_ALL", ""),
            ("LC_CTYPE", "C"),
            ("LANG", "en_US.UTF-8"),
        ])));
    }

    #[test]
    fn no_locale_set_outputs_utf8() {
        assert!(locale_is_utf8(environment(&[])));
        assert!(locale_is_utf8(environment(&[("LC_ALL", ""), ("LANG", "")])));
    }
}

#[cfg(windows)]
mod code_page {
    use super::super::native::code_page_is_utf8;

    const UTF8: u32 = 65001;
    const US: u32 = 437;
    const WESTERN_EUROPE: u32 = 850;
    const NO_CONSOLE: u32 = 0;

    #[test]
    fn the_utf8_code_page_outputs_utf8() {
        assert!(code_page_is_utf8(UTF8));
    }

    #[test]
    fn a_legacy_code_page_outputs_ascii() {
        assert!(!code_page_is_utf8(US));
        assert!(!code_page_is_utf8(WESTERN_EUROPE));
    }

    #[test]
    fn a_process_without_a_console_outputs_utf8() {
        assert!(code_page_is_utf8(NO_CONSOLE));
    }
}
