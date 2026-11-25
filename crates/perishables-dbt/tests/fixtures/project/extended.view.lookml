view: orders_extended {
  extends: [orders]

  dimension: channel {
    type: string
    description: "Sales channel of the order."
    sql: ${TABLE}.channel ;;
  }
}

view: ghost {
  extends: [missing_view]

  dimension: phantom {
    type: string
    description: "A field on a view extending nothing."
    sql: ${TABLE}.phantom ;;
  }
}
