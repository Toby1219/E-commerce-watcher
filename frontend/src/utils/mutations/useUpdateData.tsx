import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiRequest } from "../lib/axio_init";
import type { CurrentUser } from "../store/types"; 

export function useUpdateUser() {
  const queryClient = useQueryClient();
  type payload =  {"email":string, "username":string};
  return useMutation({
    mutationFn: async ( payload:payload ) => {
      const res = await apiRequest("post", "/auth/user/update", payload);
      return res.data.body; 
    },

    onMutate: async (payload:payload) => {
      await queryClient.cancelQueries({ queryKey: ["user"] }); 
      const previous = queryClient.getQueryData<CurrentUser>(["user"]); 

      queryClient.setQueryData<CurrentUser>(["user"], (old) =>
        old
          ? {...old, username:payload.username, email:payload.email}
          : old
      );

      return { previous }; 
    },

    onError: (_err, _vars, context) => {
      if (context?.previous) {
        queryClient.setQueryData(["user"], context.previous);
      }
    },

    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["user"] });
    },
  });
}