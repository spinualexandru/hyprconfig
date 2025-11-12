import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

export function NetworkCardSkeleton() {
	return (
		<Card>
			<CardContent className="pt-6">
				<div className="flex items-center justify-between">
					<div className="flex items-center gap-3 flex-1">
						<Skeleton className="h-10 w-10 rounded-lg" />
						<div className="flex-1 space-y-2">
							<Skeleton className="h-5 w-32" />
							<Skeleton className="h-4 w-20" />
							<Skeleton className="h-3 w-28" />
						</div>
					</div>
					<div className="flex items-center gap-2">
						<Skeleton className="h-6 w-16" />
						<Skeleton className="h-10 w-10 rounded-md" />
					</div>
				</div>
			</CardContent>
		</Card>
	);
}

export function WifiCardSkeleton() {
	return (
		<Card>
			<CardContent className="pt-6">
				<div className="flex items-center justify-between">
					<div className="flex items-center gap-3 flex-1">
						<Skeleton className="h-10 w-10 rounded-lg" />
						<div className="flex-1 space-y-2">
							<Skeleton className="h-5 w-40" />
							<Skeleton className="h-4 w-32" />
						</div>
					</div>
					<div className="text-right space-y-1">
						<Skeleton className="h-4 w-12" />
						<Skeleton className="h-3 w-16" />
					</div>
				</div>
			</CardContent>
		</Card>
	);
}
