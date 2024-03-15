import Axios from "axios";

export function createRNEngineConnector(url: string, schema: string, logCallback: (msg: string) => void) {
  const port = "3000";
  const baseIP = "192.168.0.14";
  const deviceUrl = `http://${baseIP}:${port}`;
  const axios = Axios.create({
    baseURL: deviceUrl,
    headers: {
      "Content-Type": "application/json",
    },
    transformResponse: (r) => r,
  });

  // axios.get("/ping").then(() => {
  //   console.error(`✅ Connection to RN device successful! URL: ${deviceUrl}`);
  // }).catch(() => {
  //   throw new Error(`Could not ping device! Check server is runing on IP: ${deviceUrl}`)
  // })

  return {
    connect: async () => {
      const res = await axios.post(`/connect`, {
        schema,
      });
      return res.data;
    },
    query: async (
      body: string,
      trace: string,
      txId: string
    ): Promise<string> => {
      const res = await axios.post("/query", {
        body,
        trace,
        txId,
      });

      const response = JSON.parse(res.data)

      if(response.logs.length) {
        response.logs.forEach(logCallback)
      }

      return response.engineResponse;
    },
    startTransaction: async (body: string, trace: string): Promise<string> => {
      const res = await axios.post("/start_transaction", {
        body,
        trace,
      });
      // console.error("start transaction data", res.data);
      return res.data;
    },
    commitTransaction: async (txId: string, trace: string): Promise<string> => {
      const res = await axios.post("/commit_transaction", {
        txId,
        trace,
      });
      // console.error(`🐲 ${res.data}`);
      return res.data;
    },
    rollbackTransaction: async (
      txId: string,
      trace: string
    ): Promise<string> => {
      const res = await axios.post("/rollback_transaction", {
        txId,
        trace,
      });
      return res.data;
    },
    disconnect: async (trace: string) => {
      await axios.post("/disconnect", {
        trace,
      });
    },
  };
}
