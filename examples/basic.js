const result = await fetch("https://dayjs.fenxianglu.cn/", { method: "GET" });

Deno.core.print(`${await result.text()}`)

// Deno.core.print(`${JSON.stringify(result)}`);
// Deno.core.print(`status: ${result.status}\n`);
// Deno.core.print(`headers: ${JSON.stringify(result.headers)}`);
// Deno.core.print(`body: ${result.body}`);

