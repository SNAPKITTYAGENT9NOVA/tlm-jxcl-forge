const express = require("express");
const app = express();

const PORT = process.env.PORT || 3000;

app.get("/photos", async (req, res) => {
  const start = Date.now();
  const response = await fetch("https://jsonplaceholder.typicode.com/photos");
  const data = await response.json();
  const duration = Date.now() - start;
  console.log(`API Call took ${duration}ms`);

  res.json({
    duration: `${duration}ms`,
    count: data.length,
    data: data.slice(0, 3),
  });
});

app.listen(PORT, () => {
  console.log(`Server is running on port ${PORT}`);
});