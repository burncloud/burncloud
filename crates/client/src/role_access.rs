/// Keep client role interpretation aligned with the current BurnCloud server.
/// The persisted default roles are `admin` and `user`, and provider-management
/// authorization currently accepts the `admin` role explicitly.
pub fn is_staff_role(role: &str) -> bool {
    role.trim().eq_ignore_ascii_case("admin")
}

pub fn is_staff_roles(roles: &[String]) -> bool {
    roles.iter().any(|role| is_staff_role(role))
}
