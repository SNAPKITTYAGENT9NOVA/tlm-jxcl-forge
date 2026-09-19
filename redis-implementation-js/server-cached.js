const express = require("express");
const { createClient } = require("redis");
const app = express();

const PORT = process.env.PORT || 3001;

const client = createClient({ url: "redis://127.0.0.1:6379" });
client.on("error", (err) => console.error(`Redis conn error ${err}`));

(async () => {
  try {
    await client.connect();
  } catch (err) {
    console.error(`Failed to connect to Redis: ${err.message}`);
    process.exit(1);
  }

  app.get("/photos", async (req, res) => {
    const start = Date.now();
    const cacheKey = "photos";

    try {
      const cached = await client.get(cacheKey);
      if (cached) {
        const duration = Date.now() - start;
        console.log(`Cache HIT (${duration}ms)`);
        const parsed = JSON.parse(cached);

        return res.json({
          duration: `${duration}ms`,
          count: parsed.length,
          sample: parsed.slice(0, 3),
        });
      }
      const response = await fetch(
        "https://jsonplaceholder.typicode.com/photos",
      );
      const data = await response.json();

      await client.setEx(cacheKey, 3600, JSON.stringify(data));
      const duration = Date.now() - start;
      console.log(`API Call took ${duration}ms`);

      res.json({
        duration: `${duration}ms`,
        count: data.length,
        data: data.slice(0, 3),
      });
    } catch (err) {
      console.error(`Error fetching data ${err}`);
      res.status(500).json({ error: "Internal error" });
    }
  });

  app.listen(PORT, () => {
    console.log(`Server is running on port ${PORT}`);
  });
})();