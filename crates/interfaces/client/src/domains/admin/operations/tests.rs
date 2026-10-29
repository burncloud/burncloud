use super::model::{AdminOperationsModel, OperationsLogLevel};
use crate::app::router::routes::Route;

#[test]
fn mock_values_match_operations_reference_page() {
    let model = AdminOperationsModel::mock();

    assert_eq!(model.logs.len(), 3);
    assert_eq!(model.logs[0].category, "Capacity");
    assert_eq!(model.logs[0].level, OperationsLogLevel::Action);
    assert_eq!(model.logs[0].time, "2 mins ago");
    assert_eq!(model.logs[1].category, "Failover");
    assert_eq!(
        model.logs[1].title,
        "Isolated unstable Node ap-hkg-03 due to PCIe thermal warning"
    );
    assert_eq!(model.logs[2].category, "Economics");
    assert_eq!(
        model.logs[2].action_taken,
        "Updated dynamic batching parameters."
    );
}

#[test]
fn admin_operations_route_parses_to_expected_variant() {
    assert!(matches!(
        "/admin/operations".parse::<Route>(),
        Ok(Route::AdminOperations {})
    ));
}
