view: orders {
  dimension: region {
    type: string
    description: "Sales region of the order."
    sql: ${TABLE}.region ;;
  }

  measure: margin {
    type: sum
    description: "Gross margin."
    sql: ${gross_profit} ;;
  }
}
