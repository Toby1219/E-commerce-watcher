import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiRequest } from "../lib/axio_init";
import type { Bookmarks, History } from "../store/types";

// Bookmark
export function useDeleteBookmark() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: async (productId: number) => {
            const resp = await apiRequest("post", "/api/user/del/bookmark", { product_id: productId });
            return resp.data.body;
        },
        onMutate: async (productId: number) => {
            await queryClient.cancelQueries({ queryKey: ["bookmark"] });
            const previous = queryClient.getQueryData<Bookmarks>(["bookmark"]);

            queryClient.setQueryData<Bookmarks>(["bookmark"], (old) => 
                old ? { total: old.total - 1, products: old.products.filter((p) => p.id !== productId)} : old
            );
            return { previous };
        },
        onError: (_err, _productId, context) => {
            if (context?.previous) {
                queryClient.setQueryData(["bookmark"], context.previous);
            }
        },
        onSettled: () => {
            queryClient.invalidateQueries({ queryKey: ["bookmark"] });
        },
    });
}

// History
export function useDeleteHistory() {
    const queryClient = useQueryClient();
    type payload = {phrase: string};

    return useMutation({
        mutationFn: async (payload:payload) => {
            const resp = await apiRequest("post", "/api/del/history", {phrase: payload.phrase});
            return resp.data.body;
        },
        onMutate: async (payload:payload) => {
            await queryClient.cancelQueries({ queryKey: ["history"] });
            const previous = queryClient.getQueryData<History>(["history"]);

            queryClient.setQueryData<History>(["history"], (old) => 
                old ? { total: old.total - 1, search: old.search.filter((s) => s.phrase !== payload.phrase)} : old
            );
            return { previous };
        },
        onError: (_err, _productId, context) => {
            if (context?.previous) {
                queryClient.setQueryData(["history"], context.previous);
            }
        },
        onSettled: () => {
            queryClient.invalidateQueries({ queryKey: ["history"] });
        },
    });
}