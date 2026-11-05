import { useQuery } from "@tanstack/react-query";

import { apiRequest } from "../lib/axio_init";
import { type CurrentUser } from "../store/types";

export function useCurrentUser() {
    return useQuery({
        queryKey: ["user"],
        queryFn: async () => {
            const resp = await apiRequest("get", "/api/user/me");
            return resp.data.body as CurrentUser;
        },
    });
}