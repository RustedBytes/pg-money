use pgrx::prelude::*;

#[allow(unreachable_code)]
pub(crate) fn fail_input(reason: &str) -> ! {
    ereport!(
        ERROR,
        PgSqlErrorCode::ERRCODE_INVALID_TEXT_REPRESENTATION,
        format!("invalid input syntax for type money_with_currency: {reason}"),
        "Use a locale-independent form such as 'USD 123.45' or 'BTC 1.00000000'."
    );
    unreachable!()
}

#[allow(unreachable_code)]
pub(crate) fn fail_parameter(reason: &str) -> ! {
    ereport!(
        ERROR,
        PgSqlErrorCode::ERRCODE_INVALID_PARAMETER_VALUE,
        reason.to_owned()
    );
    unreachable!()
}

#[allow(unreachable_code)]
pub(crate) fn fail_binary(reason: &str) -> ! {
    ereport!(
        ERROR,
        PgSqlErrorCode::ERRCODE_INVALID_BINARY_REPRESENTATION,
        format!("invalid binary representation for type money_with_currency: {reason}")
    );
    unreachable!()
}
