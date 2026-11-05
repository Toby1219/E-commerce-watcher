import { useQuery } from "@tanstack/react-query";

import { apiRequest } from "../lib/axio_init";
import { type Bookmarks } from "../store/types";
import { useCurrentUser } from "./user";

export function useBookmark() {
    const { data: user } = useCurrentUser();

    return useQuery({
        queryKey: ["bookmark"],
        queryFn: async () => {
            const resp = await apiRequest("get", "/api/user/bookmark");
            return resp.data.body as Bookmarks;
        },
        enabled: !!user,
    });
}