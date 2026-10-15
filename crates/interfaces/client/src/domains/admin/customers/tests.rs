use super::model::{AdminCustomersModel, CustomerRisk};
use crate::app::router::routes::Route;

#[test]
fn mock_customer_rows_match_the_reference_page() {
    let model = AdminCustomersModel::mock();

    assert_eq!(model.customers.len(), 4);
    assert_eq!(model.customers[0].name, "HyperScale AI Labs");
    assert_eq!(model.customers[0].email, "infra@hyperscale.ai");
    assert_eq!(model.customers[0].keys, 8);
    assert_eq!(model.customers[0].spend, "$48,200");
    assert_eq!(model.customers[0].balance, "$18,400.00");
    assert_eq!(model.customers[0].risk, CustomerRisk::Low);
    assert_eq!(model.customers[3].risk, CustomerRisk::LowBalance);
}

#[test]
fn admin_customers_route_parses_to_the_expected_variant() {
    assert!(matches!(
        "/admin/customers".parse::<Route>(),
        Ok(Route::AdminCustomers {})
    ));
}
