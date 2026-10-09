# burncloud-identity-user

Identity-owned vertical User crate.

This crate consolidates the former `burncloud-database-user` and
`burncloud-service-user` technical layers under one business Owner.

It owns the existing account/role/API-key/recharge/password-reset persistence
and the existing registration, login, bcrypt, JWT and traffic-class behavior.

This migration does not change schema, migrations, credential projection,
balance semantics, authentication semantics, or HTTP contracts.

The Cargo package is `burncloud-identity-user`. During #793 the Rust library
target intentionally remains `burncloud_service_user` so structural migration
is isolated from the repository-wide import rename.
