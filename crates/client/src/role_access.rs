pub fn is_staff_role(role: &str) -> bool {
    matches!(
        role.trim().to_ascii_lowercase().as_str(),
        "root" | "admin" | "administrator" | "operator" | "owner"
    )
}

pub fn is_staff_roles(roles: &[String]) -> bool {
    roles.iter().any(|role| is_staff_role(role))
}
