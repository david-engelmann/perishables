view: orders {
  sql_table_name: sales.orders ;;
  dimension: amount {
    type: number
    label: "Amount"
    synonyms: ["order_amount"]
    sql: ${TABLE}.amount ;;
  }
  measure: total_revenue {
    type: sum
    label: "Total Revenue"
    sql: ${amount} ;;
  }
}

explore: orders {
  join: customers {
    sql_on: ${orders.customer_id} = ${customers.id} ;;
  }
}
