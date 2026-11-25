cube(`orders`, {
  sql: `SELECT * FROM sales.orders`,
  measures: {
    revenue: {
      type: `sum`,
      sql: `amount`,
      title: `Revenue`,
      description: `Gross revenue.`,
      meta: {
        ai_context: `Revenue is gross of refunds.`
      }
    }
  },
  dimensions: {
    region: {
      type: `string`,
      sql: `region`,
      title: `Region`,
      description: `Sales region.`
    }
  },
  joins: {
    customers: {
      sql: `${CUBE}.customer_id = ${customers}.id`,
      relationship: `many_to_one`
    }
  }
});

cube(`customers`, {
  sql: `SELECT * FROM sales.customers`,
  dimensions: {
    name: {
      type: `string`,
      sql: `name`,
      title: `Name`,
      description: `Customer name.`
    }
  }
});

view(`sales`, {
  cubes: [
    {
      join_path: orders,
      includes: [`revenue`, `region`]
    },
    {
      join_path: `orders.customers`,
      includes: `*`
    }
  ]
});
