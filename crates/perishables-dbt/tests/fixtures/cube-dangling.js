cube(`orders`, {
  measures: {
    revenue: {
      type: `sum`,
      sql: `amount`,
      title: `Revenue`,
      description: `Gross revenue.`
    }
  },
  joins: {
    shipments: {
      sql: `${CUBE}.id = ${shipments}.order_id`,
      relationship: `one_to_many`
    }
  }
});
