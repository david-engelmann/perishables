view: customers {
  dimension: id {
    type: number
    description: "Customer identifier."
    sql: ${TABLE}.id ;;
  }

  dimension: name {
    type: string
    description: "Customer name."
    sql: ${TABLE}.name ;;
  }
}
