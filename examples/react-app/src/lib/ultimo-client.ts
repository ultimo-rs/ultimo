export interface JsonRpcError {
  code: number;
  message: string;
  data?: unknown;
}

export class JsonRpcClientError extends Error {
  constructor(public readonly error: JsonRpcError) {
    super(error.message);
    this.name = "JsonRpcClientError";
  }
}

export interface User {
  id: number;
  name: string;
  email: string;
}

export interface UserListResponse {
  users: User[];
  total: number;
}

export class UltimoRpcClient {
  private _idCounter = 0;

  constructor(private baseUrl: string = "/api") {}

  private nextId(): number {
    return ++this._idCounter;
  }

  private async call<T>(method: string, params: unknown): Promise<T> {
    const id = this.nextId();
    const response = await fetch(this.baseUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ jsonrpc: "2.0", method, params, id }),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const data = await response.json();
    if (data.error) {
      throw new JsonRpcClientError(data.error);
    }
    return data.result as T;
  }

  async listUsers(params: Record<string, unknown>): Promise<UserListResponse> {
    return this.call("listUsers", params);
  }

  async getUser(params: { id: number }): Promise<User> {
    return this.call("getUser", params);
  }

  async createUser(params: { name: string; email: string }): Promise<User> {
    return this.call("createUser", params);
  }
}
