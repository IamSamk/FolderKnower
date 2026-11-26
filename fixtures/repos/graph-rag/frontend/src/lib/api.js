export const req = (path, body) => fetch(path, { method: "POST", body: JSON.stringify(body) });
export const ask = (question) => req("/api/ask", { question });
