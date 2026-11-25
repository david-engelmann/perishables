view: orders {
  dimension: amount {
    type: number
    description: "Gross amount of the order."
    sql: ${TABLE}.amount ;;
  }

  dimension: region {
    type: string
    description: "Sales region of the order."
    sql: ${TABLE}.region ;;
  }

  measure: total_revenue {
    type: sum
    description: "Gross revenue."
    sql: ${amount} ;;
  }

  measure: customer_label {
    type: string
    description: "The ordering customer's name."
    sql: ${customers.name} ;;
  }
}
