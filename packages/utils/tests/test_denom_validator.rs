use mars_utils::{error::ValidationError::InvalidDenom, helpers::validate_native_denom};

#[test]
fn length_below_three() {
    let res = validate_native_denom("su");
    assert_eq!(
        res,
        Err(InvalidDenom {
            reason: "Invalid denom length".to_string()
        }),
    )
}

#[test]
fn length_above_128() {
    let res =
        validate_native_denom("fadjkvnrufbaalkefoi2934095sfonalf89o234u2sadsafsdbvsdrgweqraefsdgagqawfaf104hqflkqehf98348qfhdsfave3r23152wergfaefegqsacasfasfadvcadfsdsADsfaf324523");
    assert_eq!(
        res,
        Err(InvalidDenom {
            reason: "Invalid denom length".to_string()
        }),
    )
}

#[test]
fn first_char_not_alphabetical() {
    let res = validate_native_denom("7asdkjnfe7");
    assert_eq!(
        res,
        Err(InvalidDenom {
            reason: "First character is not ASCII alphabetic".to_string()
        }),
    )
}

#[test]
fn invalid_character() {
    let res = validate_native_denom("fakjfh&asd!#");
    assert_eq!(
        res,
        Err(InvalidDenom {
            reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                .to_string()
        }),
    )
}

#[test]
fn correct_denom() {
    let res = validate_native_denom("umars");
    assert_eq!(res, Ok(()));

    let res = validate_native_denom(
        "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2",
    );
    assert_eq!(res, Ok(()));
}
