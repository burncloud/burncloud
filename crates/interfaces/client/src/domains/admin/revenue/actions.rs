use super::model::AdminRevenueModel;

fn escape_csv(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub fn to_csv(model: &AdminRevenueModel) -> String {
    let mut csv =
        String::from("Model,Tokens,GMV,Supplier Payout,Platform Net Revenue,Net Margin\r\n");

    for row in model.rows {
        csv.push_str(
            &[
                row.model,
                row.tokens,
                row.gmv,
                row.supplier_payout,
                row.net_revenue,
                row.net_margin,
            ]
            .into_iter()
            .map(escape_csv)
            .collect::<Vec<_>>()
            .join(","),
        );
        csv.push_str("\r\n");
    }

    csv
}

pub fn download_csv(csv: &str) {
    let Ok(serialized) = serde_json::to_string(csv) else {
        return;
    };

    let script = format!(
        "const csv = {serialized}; \
         const blob = new Blob([\"\\uFEFF\", csv], {{ type: \"text/csv;charset=utf-8;\" }}); \
         const url = URL.createObjectURL(blob); \
         const link = document.createElement(\"a\"); \
         link.href = url; \
         link.download = \"platform-revenue.csv\"; \
         document.body.appendChild(link); \
         link.click(); \
         link.remove(); \
         URL.revokeObjectURL(url);"
    );
    dioxus::document::eval(&script);
}
