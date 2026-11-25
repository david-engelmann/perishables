connection: "sales_warehouse"

include: "/project/*.view.lookml"
include: "/dashboards/*.lookml"

explore: orders {
  join: customers {
    type: left_outer
    relationship: many_to_one
    sql_on: ${orders.customer_id} = ${customers.id} ;;
  }
}
