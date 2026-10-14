use super::{
    actions::to_csv,
    model::{AdminRevenueModel, AdminRevenueRow},
};
use crate::app::router::routes::Route;

#[test]
fn reference_values_match_the_admin_revenue_page() {
    let model = AdminRevenueModel::reference();

    assert_eq!(model.gmv, "$384,200");
    assert_eq!(model.supplier_payouts, "$259,720");
    assert_eq!(model.net_revenue, "$124,480");
    assert_eq!(model.prepaid_credits, "$842,100");
    assert_eq!(model.rows.len(), 4);
    assert_eq!(
        model.rows.map(|row| (
            row.model,
            row.tokens,
            row.gmv,
            row.supplier_payout,
            row.net_revenue,
            row.net_margin
        )),
        [
            (
                "DeepSeek V3 (671B)",
                "1,420.0M",
                "$148,200",
                "$98,400",
                "$49,800",
                "33.6%"
            ),
            (
                "DeepSeek R1 Reasoning",
                "680.0M",
                "$112,400",
                "$74,200",
                "$38,200",
                "34.0%"
            ),
            (
                "Qwen 2.5 72B",
                "420.0M",
                "$64,100",
                "$44,800",
                "$19,300",
                "30.1%"
            ),
            (
                "Claude 3.5 Sonnet Pass-Through",
                "140.0M",
                "$59,500",
                "$42,320",
                "$17,180",
                "28.9%"
            ),
        ]
    );
}

#[test]
fn revenue_csv_contains_all_rows_and_escapes_fields() {
    let mut model = *AdminRevenueModel::reference();
    model.rows[0] = AdminRevenueRow {
        model: "Model, \"special\"",
        tokens: "1.0M",
        gmv: "$10",
        supplier_payout: "$6",
        net_revenue: "$4",
        net_margin: "40%",
    };

    let csv = to_csv(&model);

    assert!(csv.starts_with(
        "Model,Tokens,GMV,Supplier Payout,Platform Net Revenue,Net Margin\r\n"
    ));
    assert_eq!(csv.lines().count(), 5);
    assert!(csv.contains("\"Model, \"\"special\"\"\""));
    assert!(csv.contains("\"$38,200\""));
    assert!(csv.contains("\"28.9%\""));
}

#[test]
fn admin_revenue_route_parses_to_the_dedicated_page() {
    assert!(matches!(
        "/admin/revenue".parse::<Route>(),
        Ok(Route::AdminRevenue {})
    ));
}
