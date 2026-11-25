cube(`orders`, {
  measures: {
    revenue: {
      type: `sum`,
      sql: `amount`,
      title: `Revenue`,
      description: `Gross revenue.`
    }
  }
});

view(`sales`, {
  cubes: [
    {
      join_path: orders,
      includes: [`revenue`, `gross_profit`]
    }
  ]
});
