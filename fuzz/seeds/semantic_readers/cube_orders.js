cube(`Orders`, {
  sql_table: `public.orders`,
  joins: {
    Customers: {
      relationship: `many_to_one`,
      sql: `${CUBE}.customer_id = ${Customers}.id`
    }
  },
  measures: {
    revenue: {
      type: `sum`,
      sql: `amount`,
      title: `Revenue`,
      description: `Gross order revenue`
    }
  },
  dimensions: {
    region: {
      type: `string`,
      sql: `region`,
      title: `Region`
    }
  }
});
