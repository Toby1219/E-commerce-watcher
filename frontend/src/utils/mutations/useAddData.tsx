import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiRequest } from "../lib/axio_init";
import type { History, Bookmarks, Products } from "../store/types"; // your existing type

export function useAddHistory() {
  const queryClient = useQueryClient();
  type payload = { phrase: string };
  return useMutation({
    mutationFn: async ( payload:payload ) => {
      const res = await apiRequest("post", "/api/history", { phrase: payload.phrase });
      return res.data.body; 
    },

    onMutate: async (payload:payload) => {
      await queryClient.cancelQueries({ queryKey: ["history"] }); // stop a racing refetch from clobbering this
      const previous = queryClient.getQueryData<History>(["history"]); // snapshot for rollback

      const optimisticEntry = {
        phrase: payload.phrase,
        created_at: new Date().toISOString(), // placeholder — server will give the real one on refetch
      };

      queryClient.setQueryData<History>(["history"], (old) =>
        old
          ? { ...old, total: old.total + 1, search: [optimisticEntry, ...old.search] }
          : { total: 1, search: [optimisticEntry] }
      ); // instantly shows in UI, before the request even resolves

      return { previous }; // pass forward for onError
    },

    onError: (_err, _vars, context) => {
      if (context?.previous) {
        queryClient.setQueryData(["history"], context.previous); // undo the optimistic insert if the API call failed
      }
    },

    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["history"] });
    },
  });
}



export function useAddBookmark() {
  const queryClient = useQueryClient();
  
  return useMutation({
    mutationFn: async ( product:Products ) => {
      const res = await apiRequest("post", "/api/user/bookmark", { product_id: product.id });
      return res.data.body; 
    },

    onMutate: async (product:Products) => {
      await queryClient.cancelQueries({ queryKey: ["bookmark"] }); // stop a racing refetch from clobbering this
      const previous = queryClient.getQueryData<Bookmarks>(["bookmark"]); // snapshot for rollback

      queryClient.setQueryData<Bookmarks>(["bookmark"], (old) => {
          if (!old) return {total:1, products: [product]};
          if (old.products.some((p) => p.id === product.id)) return old;
          return {
            total: old.total + 1,
            products: [product, ...old.products],
          }
      });

      return { previous }; // pass forward for onError
    },

    onError: (_err, _vars, context) => {
      if (context?.previous) {
        queryClient.setQueryData(["bookmark"], context.previous); // undo the optimistic insert if the API call failed
      }
    },

    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["bookmark"] });
    },
  });
}
