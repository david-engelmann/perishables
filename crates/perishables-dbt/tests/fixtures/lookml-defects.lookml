view: orders {
  dimension: region {
    type: string
    label: "Region"
    description: "Sales region of the order."
    synonyms: [revenue]
    sql: ${TABLE}.region ;;
  }

  measure: total_revenue {
    type: sum
    label: "Revenue"
    description: "Gross revenue."
    sql: ${gross_profit} ;;
  }
}

explore: orders {
  join: shipments {
    type: left_outer
    relationship: one_to_many
    sql_on: ${orders.id} = ${shipments.order_id} ;;
  }
}
