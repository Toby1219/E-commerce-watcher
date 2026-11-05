import { useQuery } from "@tanstack/react-query";

import { apiRequest } from "../lib/axio_init";
import { type History } from "../store/types";
// import { useCurrentUser } from "./user";


export function useHistory() {
    // const { data: user } = useCurrentUser();

    return useQuery({
        queryKey: ["history"],
        queryFn: async () => {
            const resp = await apiRequest("get", "/api/history");
            return resp.data.body as History;
        },
    });
}
