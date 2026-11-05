import axios from "axios";

const BASE_URL = "http://127.0.0.1:8181"
const REFRESH_URL = "/auth/refresh"

export const api = axios.create({
    baseURL: BASE_URL,
    timeout: 10000,
    headers: {
        'Content-Type': 'application/json'
    },
    withCredentials: true
});

// Plain instance, deliberately with NO interceptor — used only for refresh
const refreshApi = axios.create({
    baseURL: BASE_URL,
    withCredentials: true,
});

api.interceptors.response.use(
    (response) => response,
    async (error) => {
        const originalRequest = error.config;

        if (error.response?.status === 401 && !originalRequest._retry) {
            originalRequest._retry = true;

            try {
                await refreshApi.post(REFRESH_URL);
                return api(originalRequest);
            } catch (err) {
                // window.location.href = "/login";
                return Promise.reject(err);
            }
        }
        return Promise.reject(error);
    }
);

export const apiRequest = async (
    method: "get" | "post" | "patch" | "delete",
    url: string,
    body?:object
) => {
    const response = await api({
        method,
        url,
        data: body
    });
    return response

}