import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

export function AudioDeviceSkeleton() {
	return (
		<Card>
			<CardContent className="pt-4">
				<div className="flex items-start gap-3">
					<Skeleton className="h-10 w-10 rounded-lg shrink-0" />
					<div className="flex-1">
						<Skeleton className="h-5 w-48 mb-2" />
						<Skeleton className="h-4 w-24 mb-3" />
						<div className="flex items-center gap-3">
							<Skeleton className="h-8 w-8 rounded-md" />
							<Skeleton className="h-2 flex-1 rounded-full" />
							<Skeleton className="h-4 w-12" />
						</div>
					</div>
				</div>
			</CardContent>
		</Card>
	);
}

export function AudioSectionSkeleton({ count = 2 }: { count?: number }) {
	return (
		<div className="space-y-3">
			{Array.from({ length: count }).map((_, i) => (
				<AudioDeviceSkeleton key={`skeleton-${i}`} />
			))}
		</div>
	);
}
