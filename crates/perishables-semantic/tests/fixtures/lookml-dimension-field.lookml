view: orders {
  sql_table_name: sales.orders ;;

  dimension: amount {
    type: number
    sql: ${TABLE}.amount ;;
  }

  dimension: amount_with_tax {
    type: number
    sql: ${amount} * 1.2 ;;
  }

  dimension: margin_band {
    type: string
    sql: CASE WHEN ${gross_margin} > 0.2 THEN 'high' ELSE 'low' END ;;
  }
}
