// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { useOnScreen } from '@mysten/core';
import { useRef, useEffect } from 'react';
import { Link } from 'react-router-dom';

import { useActiveAddress } from '_app/hooks/useActiveAddress';
import Alert from '_components/alert';
import { ErrorBoundary } from '_components/error-boundary';
import Loading from '_components/loading';
import LoadingSpinner from '_components/loading/LoadingIndicator';
import { NFTDisplayCard } from '_components/nft-display';
import { ampli } from '_src/shared/analytics/ampli';
import { useGetNFTs } from '_src/ui/app/hooks/useGetNFTs';
import PageTitle from '_src/ui/app/shared/PageTitle';

function NftsPage() {
	const accountAddress = useActiveAddress();
	const {
		data: nfts,
		hasNextPage,
		isInitialLoading,
		isFetchingNextPage,
		error,
		isLoading,
		fetchNextPage,
		isError,
	} = useGetNFTs(accountAddress);
	const observerElem = useRef<HTMLDivElement | null>(null);
	const { isIntersecting } = useOnScreen(observerElem);
	const isSpinnerVisible = isFetchingNextPage && hasNextPage;

	useEffect(() => {
		if (isIntersecting && hasNextPage && !isFetchingNextPage) {
			fetchNextPage();
		}
	}, [nfts.ownedObjects.length, isIntersecting, fetchNextPage, hasNextPage, isFetchingNextPage]);
	if (isInitialLoading) {
		return (
			<div className="mt-1 flex w-full justify-center">
				<LoadingSpinner />
			</div>
		);
	}

	return (
		<div className="flex flex-1 flex-col flex-nowrap items-center gap-4">
			<PageTitle title="NFTs" />
			<Loading loading={isLoading}>
				{isError ? (
					<Alert>
						<div>
							<strong>Sync error (data might be outdated)</strong>
						</div>
						<small>{(error as Error).message}</small>
					</Alert>
				) : null}

				{nfts?.ownedObjects.length ? (
					<div className="grid w-full grid-cols-2 gap-x-3.5 gap-y-4">
						{nfts.kioskOwnerTokens?.map((object) => (
							<Link
								to={`/kiosk?${new URLSearchParams({
									kioskId:
										object && object.content && 'fields' in object.content
											? object.content.fields.kiosk ||
											  (object.content && 'fields' in object.content && object.content.fields.for)
											: null,
								}).toString()}`}
								onClick={() => {}}
								key={object.objectId}
								className="no-underline"
							>
								<ErrorBoundary>
									<NFTDisplayCard
										objectId={object.objectId}
										size="lg"
										showLabel
										animateHover
										borderRadius="xl"
									/>
								</ErrorBoundary>
							</Link>
						))}
						{nfts.ownedObjects.map(({ objectId, type }) => (
							<Link
								to={`/nft-details?${new URLSearchParams({
									objectId,
								}).toString()}`}
								onClick={() => {
									ampli.clickedCollectibleCard({
										objectId,
										collectibleType: type!,
									});
								}}
								key={objectId}
								className="no-underline"
							>
								<ErrorBoundary>
									<NFTDisplayCard
										objectId={objectId}
										size="lg"
										showLabel
										animateHover
										borderRadius="xl"
									/>
								</ErrorBoundary>
							</Link>
						))}
						<div ref={observerElem}>
							{isSpinnerVisible ? (
								<div className="mt-1 flex w-full justify-center">
									<LoadingSpinner />
								</div>
							) : null}
						</div>
					</div>
				) : (
					<div className="flex flex-1 items-center self-center text-caption font-semibold text-steel-darker">
						No NFTs found
					</div>
				)}
			</Loading>
		</div>
	);
}

export default NftsPage;
