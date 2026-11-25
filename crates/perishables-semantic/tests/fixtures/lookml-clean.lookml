view: orders {
  sql_table_name: sales.orders ;;

  dimension: amount {
    type: number
    description: "Gross amount of the order."
    sql: ${TABLE}.amount ;;
  }

  dimension: region {
    type: string
    label: "Region"
    description: "Sales region of the order."
    synonyms: [territory]
    sql: ${TABLE}.region ;;
  }

  measure: total_revenue {
    type: sum
    label: "Total Revenue"
    description: "Gross revenue."
    synonyms: [sales]
    sql: ${amount} ;;
  }
}

view: customers {
  dimension: name {
    type: string
    label: "Customer Name"
    description: "Customer name."
    sql: ${TABLE}.name ;;
  }
}

explore: orders {
  join: customers {
    type: left_outer
    relationship: many_to_one
    sql_on: ${orders.customer_id} = ${customers.id} ;;
  }
}
