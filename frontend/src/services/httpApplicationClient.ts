import type { ApplicationClient, ApplicationRequest, ApplicationResponse } from '../types/application';

export class HttpApplicationClient implements ApplicationClient {
  constructor(private readonly basePath = '/api') {}

  request(request: ApplicationRequest): Promise<ApplicationResponse> {
    return fetch(`${this.basePath}${request.path}`, {
      method: request.method ?? 'GET',
      headers: request.headers,
      body: request.body,
      signal: request.signal,
      cache: request.cache,
    });
  }
}
